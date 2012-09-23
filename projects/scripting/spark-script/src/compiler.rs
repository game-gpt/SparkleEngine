//! 脚本编译门面：产出制品，不持有 VM / JIT。

use std::collections::HashMap;
use std::sync::Arc;

use crate::artifact::{
    ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError,
};
use crate::cache::ArtifactCache;
use crate::dep_graph::PackageDepGraph;
use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
use crate::request::{CompilationRequest, LanguageFrontend};
use crate::{compile_module_with_registry, ScriptError, ScriptLanguage};
use spark_script_valkyrie::NativeRegistry;

/// 编译产物（目标 → 链接 → 映像）。
#[derive(Debug, Clone)]
pub struct CompiledPackage {
    pub object: SparkObject,
    pub program: LinkedProgram,
    pub image: ExecutableImage,
}

/// 编译器：驱动前端与制品管线，不执行脚本。
#[derive(Debug, Default)]
pub struct ScriptCompiler {
    pub diagnostics: crate::diagnostic::DiagnosticBatch,
    pub cache: ArtifactCache,
}

impl ScriptCompiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 按正式 [`CompilationRequest`] 编译并链接、验证（命中缓存则跳过前端）。
    pub fn compile(&mut self, request: &CompilationRequest) -> Result<CompiledPackage, ScriptError> {
        let source = request.primary_source().ok_or_else(|| {
            ScriptError::compile_reason("compilation_request_missing_source")
        })?;
        let key = ArtifactCache::key_for(request, source);
        if let Some(hit) = self.cache.get(key) {
            return Ok(hit);
        }
        let language = ScriptLanguage::from(request.language.frontend);
        let binds = request
            .host_schema
            .to_bind_table_with_policy(crate::compile_policy_from_request(request))
            .map_err(ScriptError::compile_reason)?;
        let module = match language {
            ScriptLanguage::Valkyrie => {
                spark_script_valkyrie::compile_with_binds(source, &binds)?
            }
            ScriptLanguage::Lua => spark_script_lua::compile_with_binds(source, &binds)?,
            ScriptLanguage::Ruby => spark_script_ruby::compile_with_binds(source, &binds)?,
        };
        let package = self.seal(request, module)?;
        self.cache.insert(key, package.clone());
        Ok(package)
    }

    /// 语言 + 源码 + schema 的便利入口。
    pub fn compile_source(
        &mut self,
        language: ScriptLanguage,
        source: &str,
        host: &HostSchema,
    ) -> Result<CompiledPackage, ScriptError> {
        let request = CompilationRequest::repl(language, source, host.clone());
        self.compile(&request)
    }

    /// 仅函数名列表（自动生成最小 schema 桩，走正式缓存键）。
    pub fn compile_with_native_names(
        &mut self,
        language: ScriptLanguage,
        source: &str,
        natives: &[&str],
    ) -> Result<CompiledPackage, ScriptError> {
        let host = stub_schema_from_names(natives);
        self.compile_source(language, source, &host)
    }

    /// 只编译为目标 [`SparkObject`]（不链接），供写出 `.spko` 或后续 `link_many`。
    pub fn compile_object(
        &mut self,
        request: &CompilationRequest,
    ) -> Result<SparkObject, ScriptError> {
        let source = request.primary_source().ok_or_else(|| {
            ScriptError::compile_reason("compilation_request_missing_source")
        })?;
        let language = ScriptLanguage::from(request.language.frontend);
        let binds = request
            .host_schema
            .to_bind_table_with_policy(crate::compile_policy_from_request(request))
            .map_err(ScriptError::compile_reason)?;
        let module = match language {
            ScriptLanguage::Valkyrie => {
                spark_script_valkyrie::compile_with_binds(source, &binds)?
            }
            ScriptLanguage::Lua => spark_script_lua::compile_with_binds(source, &binds)?,
            ScriptLanguage::Ruby => spark_script_ruby::compile_with_binds(source, &binds)?,
        };
        SparkObject::from_module(
            request.package.clone(),
            request.language.clone(),
            &request.host_schema,
            module,
        )
        .map_err(script_link_error)
    }

    /// 将已有目标链接并验证为完整包。
    ///
    /// 多目标时 `entry_package` 指定入口包；单目标可传该目标的 `package`。
    pub fn link_objects(
        &mut self,
        objects: &[SparkObject],
        host: &HostSchema,
        entry_package: &crate::request::PackageId,
    ) -> Result<CompiledPackage, ScriptError> {
        let program = if objects.len() == 1 {
            LinkedProgram::link_single(objects[0].clone(), host).map_err(script_link_error)?
        } else {
            LinkedProgram::link_many(objects, host, entry_package).map_err(script_link_error)?
        };
        let image = ExecutableImage::verify(program.clone()).map_err(script_verify_error)?;
        let object = objects
            .iter()
            .find(|o| {
                o.package.name == entry_package.name && o.package.version == entry_package.version
            })
            .cloned()
            .ok_or_else(|| ScriptError::compile_reason("spark.script.link.missing_entry_package"))?;
        Ok(CompiledPackage {
            object,
            program,
            image,
        })
    }

    /// 按 [`PackageDepGraph`] 拓扑序重排目标（依赖在前，不把入口挪到下标 0）。
    pub fn order_objects_for_link(
        objects: Vec<SparkObject>,
        graph: &PackageDepGraph,
    ) -> Result<Vec<SparkObject>, ScriptError> {
        let order = graph
            .topo_order()
            .map_err(|e| ScriptError::compile_reason(e.to_string()))?;
        if order.is_empty() {
            return Err(ScriptError::compile_reason("spark.script.link.empty_set"));
        }
        let mut by_key: HashMap<(Arc<str>, Arc<str>), SparkObject> = HashMap::new();
        for obj in objects {
            by_key.insert(
                (Arc::clone(&obj.package.name), Arc::clone(&obj.package.version)),
                obj,
            );
        }
        let mut ordered = Vec::with_capacity(order.len());
        for id in &order {
            let key = (Arc::clone(&id.name), Arc::clone(&id.version));
            if let Some(obj) = by_key.remove(&key) {
                ordered.push(obj);
            }
        }
        ordered.extend(by_key.into_values());
        Ok(ordered)
    }

    pub(crate) fn seal(
        &mut self,
        request: &CompilationRequest,
        module: spark_vm::Module,
    ) -> Result<CompiledPackage, ScriptError> {
        let object = SparkObject::from_module(
            request.package.clone(),
            request.language.clone(),
            &request.host_schema,
            module,
        )
        .map_err(script_link_error)?;
        let program = LinkedProgram::link_single(object.clone(), &request.host_schema)
            .map_err(script_link_error)?;
        let image = ExecutableImage::verify(program.clone()).map_err(script_verify_error)?;
        Ok(CompiledPackage {
            object,
            program,
            image,
        })
    }
}

fn stub_schema_from_names(names: &[&str]) -> HostSchema {
    let mut schema = HostSchema::new(1);
    for name in names {
        schema.insert(HostFunction::new(HostFunctionId::new("host", *name, 1)));
    }
    schema
}

fn script_link_error(err: LinkError) -> ScriptError {
    ScriptError::compile_reason(err.code())
}

fn script_verify_error(err: VerifyError) -> ScriptError {
    ScriptError::compile_reason(err.code())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::CompilationRequest;
    use crate::ScriptLanguage;

    #[test]
    fn compile_hits_artifact_cache() {
        let host = HostSchema::new(1);
        let mut compiler = ScriptCompiler::new();
        let a = compiler
            .compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host)
            .unwrap();
        assert_eq!(compiler.cache.misses, 1);
        assert_eq!(compiler.cache.hits, 0);
        let b = compiler
            .compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host)
            .unwrap();
        assert_eq!(compiler.cache.hits, 1);
        assert_eq!(compiler.cache.len(), 1);
        assert_eq!(a.image.host_schema_hash, b.image.host_schema_hash);
    }

    #[test]
    fn compile_with_native_names_uses_cache() {
        let mut compiler = ScriptCompiler::new();
        let _ = compiler
            .compile_with_native_names(ScriptLanguage::Valkyrie, "return 40 + 2", &[])
            .unwrap();
        assert_eq!(compiler.cache.misses, 1);
        let _ = compiler
            .compile_with_native_names(ScriptLanguage::Valkyrie, "return 40 + 2", &[])
            .unwrap();
        assert_eq!(compiler.cache.hits, 1);
    }

    #[test]
    fn compile_object_and_link_roundtrip() {
        let host = HostSchema::new(1);
        let req = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1 + 2", host.clone());
        let mut compiler = ScriptCompiler::new();
        let obj = compiler.compile_object(&req).unwrap();
        let bytes = obj.to_spko_bytes().unwrap();
        let loaded = SparkObject::from_spko_bytes(&bytes).unwrap();
        let package = compiler
            .link_objects(&[loaded], &host, &req.package)
            .unwrap();
        let mut rt = crate::ScriptRuntime::from_image(&package.image, &host).unwrap();
        let v = rt.call_on_load_std().unwrap();
        assert_eq!(v.as_number(), Some(3.0));
    }
}

/// 用 registry 编译各前端（均经 `compile_with_registry`）。
pub fn compile_package_with_registry(
    language: ScriptLanguage,
    source: &str,
    natives: &NativeRegistry,
) -> Result<CompiledPackage, ScriptError> {
    let host = HostSchema::from_native_registry(natives);
    let module = compile_module_with_registry(language, source, natives)?;
    let request = CompilationRequest::repl(language, source, host);
    ScriptCompiler::new().seal(&request, module)
}

impl LanguageFrontend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Valkyrie => "valkyrie",
            Self::Lua => "lua",
            Self::Ruby => "ruby",
        }
    }
}
