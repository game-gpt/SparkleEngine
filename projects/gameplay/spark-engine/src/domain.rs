//! 脚本运行域：每个模组包对应一个隔离实例。
//!
//! [`ScriptDomain`] 持有已验证映像的运行时、宿主 schema 指纹、生命周期导出与预算。
//! **不**拥有 ECS [`spark_ecs`] 世界；结构变更须经后续命令缓冲在同步点提交。

use std::sync::Arc;

use spark_gc::Value;
use spark_script::{ExecutableImage, HostSchema, ScriptRuntime};
use spark_vm::HostHooks;

use crate::EngineError;

/// 每领域每帧（或每次回调）的资源预算（初版仅记录上限，耗尽策略后续补）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptBudget {
    pub instruction_limit: u64,
    pub host_call_limit: u64,
    pub allocation_limit: u64,
    pub call_depth_limit: u16,
}

impl Default for ScriptBudget {
    fn default() -> Self {
        Self {
            instruction_limit: 5_000_000,
            host_call_limit: 100_000,
            allocation_limit: 1_000_000,
            call_depth_limit: 256,
        }
    }
}

/// 模组脚本运行域（串行资源：同域执行暂不并行）。
pub struct ScriptDomain {
    pub mod_id: Arc<str>,
    pub runtime: ScriptRuntime,
    pub host_schema_hash: u64,
    pub host_abi_version: u32,
    pub lifecycle_exports: Vec<Arc<str>>,
    pub budget: ScriptBudget,
    /// 领域是否仍可被调度（trap / 预算耗尽后可置 false）。
    pub enabled: bool,
}

impl ScriptDomain {
    /// 从已验证映像与同一份 [`HostSchema`] 创建领域。
    pub fn from_image(
        mod_id: impl Into<Arc<str>>,
        image: &ExecutableImage,
        host: &HostSchema,
        budget: ScriptBudget,
    ) -> Result<Self, EngineError> {
        let runtime = ScriptRuntime::from_image(image, host).map_err(EngineError::Script)?;
        Ok(Self {
            mod_id: mod_id.into(),
            host_schema_hash: runtime.host_schema_hash(),
            host_abi_version: runtime.host_abi_version(),
            lifecycle_exports: image.lifecycle_exports.clone(),
            runtime,
            budget,
            enabled: true,
        })
    }

    /// 是否声明了某一生命周期导出。
    pub fn has_lifecycle(&self, name: &str) -> bool {
        self.lifecycle_exports.iter().any(|n| n.as_ref() == name)
    }

    /// 调用命名导出（生命周期或普通函数）。
    pub fn call(
        &mut self,
        name: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Value, EngineError> {
        if !self.enabled {
            return Err(EngineError::ScriptDomainDisabled {
                mod_id: self.mod_id.to_string(),
            });
        }
        self.runtime
            .call(name, args, host)
            .map_err(EngineError::Script)
    }

    /// 若存在则调用生命周期导出；不存在则返回 `None`（不跑隐式 `__main`）。
    pub fn call_lifecycle(
        &mut self,
        name: &str,
        args: &[Value],
        host: &mut dyn HostHooks,
    ) -> Result<Option<Value>, EngineError> {
        if !self.has_lifecycle(name) {
            return Ok(None);
        }
        self.call(name, args, host).map(Some)
    }

    /// 过渡期：执行映像入口（旧 `__main` / 顶层）。新模组应改用生命周期导出。
    pub fn eval_entry(&mut self, host: &mut dyn HostHooks) -> Result<Value, EngineError> {
        if !self.enabled {
            return Err(EngineError::ScriptDomainDisabled {
                mod_id: self.mod_id.to_string(),
            });
        }
        self.runtime.eval_with(host).map_err(EngineError::Script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_script::{
        HostFunction, HostFunctionId, ScriptCompiler, ScriptLanguage,
    };
    use spark_vm::StdHost;

    #[test]
    fn domain_loads_image_and_calls_lifecycle() {
        let mut host = HostSchema::new(1);
        host.insert(HostFunction::new(HostFunctionId::new("host", "noop", 1)));
        let source = r#"
            micro on_start() {
                return 42
            }
            return 0
            "#;
        let mut compiler = ScriptCompiler::new();
        let package = compiler
            .compile_source(ScriptLanguage::Valkyrie, source, &host)
            .unwrap();
        assert!(package
            .image
            .lifecycle_exports
            .iter()
            .any(|n| n.as_ref() == "on_start"));
        let mut domain =
            ScriptDomain::from_image("test.mod", &package.image, &host, ScriptBudget::default())
                .unwrap();
        let mut hooks = StdHost;
        let v = domain
            .call_lifecycle("on_start", &[], &mut hooks)
            .unwrap()
            .unwrap();
        assert_eq!(v.as_number(), Some(42.0));
        assert!(domain
            .call_lifecycle("on_unload", &[], &mut hooks)
            .unwrap()
            .is_none());
    }
}
