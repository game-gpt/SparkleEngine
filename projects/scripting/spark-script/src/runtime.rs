//! 脚本运行时：装载已验证映像并执行，不负责编译。

use spark_jit::JitEngine;
use spark_vm::{HostHooks, StdHost, Vm};

use crate::{
    ScriptError,
    artifact::{ExecutableImage, LinkError},
    host_schema::HostSchema,
    request::LanguageProfile,
};

/// 运行时实例：VM + 可选 JIT。由映像创建，不解析源码。
pub struct ScriptRuntime {
    /// 已装载字节码的虚拟机。
    pub vm: Vm,
    /// 热路径 JIT（调用后尝试优化）。
    pub jit: JitEngine,
    /// 映像携带的语言契约（诊断 / 调试用）。
    pub language: LanguageProfile,
    host_schema_hash: u64,
    host_abi_version: u32,
}

impl ScriptRuntime {
    /// 使用编译期同一份 schema 装载映像。
    pub fn from_image(image: &ExecutableImage, host: &HostSchema) -> Result<Self, ScriptError> {
        image.check_host_schema(host).map_err(link_to_script_error)?;
        let mut vm = Vm::new(image.clone_module());
        vm.prepare_host_slots(host.qualified_names());
        Ok(Self {
            vm,
            jit: JitEngine::new(256),
            language: image.language.clone(),
            host_schema_hash: image.host_schema_hash,
            host_abi_version: image.host_abi_version,
        })
    }

    /// 装载时锁定的宿主 schema 指纹。
    pub fn host_schema_hash(&self) -> u64 {
        self.host_schema_hash
    }

    /// 装载时锁定的宿主 ABI 版本。
    pub fn host_abi_version(&self) -> u32 {
        self.host_abi_version
    }

    /// 调用模组 `on_load` 生命周期（顶层语句块已封为目标时的入口）。
    pub fn call_on_load(&mut self, host: &mut dyn HostHooks) -> Result<spark_gc::Value, ScriptError> {
        self.call("on_load", &[], host)
    }

    /// 无宿主钩子时调用 `on_load`。
    pub fn call_on_load_std(&mut self) -> Result<spark_gc::Value, ScriptError> {
        let mut host = StdHost;
        self.call_on_load(&mut host)
    }

    /// 按导出函数名调用；成功后尝试 JIT 热路径优化。
    pub fn call(&mut self, name: &str, args: &[spark_gc::Value], host: &mut dyn HostHooks) -> Result<spark_gc::Value, ScriptError> {
        let v = self.vm.call_function(name, args, host)?;
        let _ = self.jit.optimize_hot(&mut self.vm);
        Ok(v)
    }
}

fn link_to_script_error(err: LinkError) -> ScriptError {
    ScriptError::compile_reason(err.code())
}
