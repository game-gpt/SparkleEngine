//! 脚本编译门面：产出制品，不持有 VM / JIT。

use std::{collections::HashMap, sync::Arc};

use crate::{
    ScriptError, ScriptLanguage,
    artifact::{ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError},
    cache::ArtifactCache,
    dep_graph::PackageDepGraph,
    host_schema::HostSchema,
    request::{CompilationRequest, LanguageFrontend},
};

/// 编译产物（目标 → 链接 → 映像）。
#[derive(Debug, Clone)]
pub struct CompiledPackage {
    /// 可重定位目标（可写出 `.spko`）。
    pub object: SparkObject,
    /// 已链接程序（槽位与生命周期已固定）。
    pub program: LinkedProgram,
    /// 已验证可执行映像（可写出 `.spkx` 或装载运行时）。
    pub image: ExecutableImage,
}

/// 编译器：驱动前端与制品管线，不执行脚本。
#[derive(Debug, Default)]
pub struct ScriptCompiler {
    /// 本会话累计诊断（当前管线主要经 [`ScriptError`] 返回致命错误）。
    pub diagnostics: crate::diagnostic::DiagnosticBatch,
    /// 按请求指纹缓存完整 [`CompiledPackage`]。
    pub cache: ArtifactCache,
}

impl ScriptCompiler {
    /// 空编译器（诊断批次与缓存均为默认）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按正式 [`CompilationRequest`] 编译并链接、验证（命中缓存则跳过前端）。
    pub fn compile(&mut self, request: &CompilationRequest) -> Result<CompiledPackage, ScriptError> {
        let source = request.primary_source().ok_or_else(|| ScriptError::compile_reason("compilation_request_missing_source"))?;
        let key = ArtifactCache::key_for(request, source);
        if let Some(hit) = self.cache.get(key) {
            return Ok(hit);
        }
        let language = ScriptLanguage::from(request.language.frontend);
        let binds =
            request.host_schema.to_bind_table_with_policy(crate::compile_policy_from_request(request)).map_err(ScriptError::compile_reason)?;
        let module = match language {
            ScriptLanguage::Valkyrie => spark_script_valkyrie::compile_with_binds(source, &binds)?,
            ScriptLanguage::Lua => spark_script_lua::compile_with_binds(source, &binds)?,
            ScriptLanguage::Ruby => spark_script_ruby::compile_with_binds(source, &binds)?,
        };
        let package = self.seal(request, module)?;
        self.cache.insert(key, package.clone());
        Ok(package)
    }

    /// 语言 + 源码 + schema 的便利入口。
    pub fn compile_source(&mut self, language: ScriptLanguage, source: &str, host: &HostSchema) -> Result<CompiledPackage, ScriptError> {
        let request = CompilationRequest::repl(language, source, host.clone());
        self.compile(&request)
    }

    /// 只编译为目标 [`SparkObject`]（不链接），供写出 `.spko` 或后续 `link_many`。
    pub fn compile_object(&mut self, request: &CompilationRequest) -> Result<SparkObject, ScriptError> {
        let source = request.primary_source().ok_or_else(|| ScriptError::compile_reason("compilation_request_missing_source"))?;
        let language = ScriptLanguage::from(request.language.frontend);
        let binds =
            request.host_schema.to_bind_table_with_policy(crate::compile_policy_from_request(request)).map_err(ScriptError::compile_reason)?;
        let module = match language {
            ScriptLanguage::Valkyrie => spark_script_valkyrie::compile_with_binds(source, &binds)?,
            ScriptLanguage::Lua => spark_script_lua::compile_with_binds(source, &binds)?,
            ScriptLanguage::Ruby => spark_script_ruby::compile_with_binds(source, &binds)?,
        };
        SparkObject::from_module(request.package.clone(), request.language.clone(), &request.host_schema, module).map_err(script_link_error)
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
        }
        else {
            LinkedProgram::link_many(objects, host, entry_package).map_err(script_link_error)?
        };
        let image = ExecutableImage::verify(program.clone()).map_err(script_verify_error)?;
        let object = objects
            .iter()
            .find(|o| o.package.name == entry_package.name && o.package.version == entry_package.version)
            .cloned()
            .ok_or_else(|| ScriptError::compile_reason("spark.script.link.missing_entry_package"))?;
        Ok(CompiledPackage { object, program, image })
    }

    /// 按 [`PackageDepGraph`] 拓扑序重排目标（依赖在前，不把入口挪到下标 0）。
    pub fn order_objects_for_link(objects: Vec<SparkObject>, graph: &PackageDepGraph) -> Result<Vec<SparkObject>, ScriptError> {
        let order = graph.topo_order().map_err(|e| ScriptError::compile_reason(e.to_string()))?;
        if order.is_empty() {
            return Err(ScriptError::compile_reason("spark.script.link.empty_set"));
        }
        let mut by_key: HashMap<(Arc<str>, Arc<str>), SparkObject> = HashMap::new();
        for obj in objects {
            by_key.insert((Arc::clone(&obj.package.name), Arc::clone(&obj.package.version)), obj);
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

    pub(crate) fn seal(&mut self, request: &CompilationRequest, module: spark_vm::Module) -> Result<CompiledPackage, ScriptError> {
        let object = SparkObject::from_module(request.package.clone(), request.language.clone(), &request.host_schema, module)
            .map_err(script_link_error)?;
        let program = LinkedProgram::link_single(object.clone(), &request.host_schema).map_err(script_link_error)?;
        let image = ExecutableImage::verify(program.clone()).map_err(script_verify_error)?;
        Ok(CompiledPackage { object, program, image })
    }
}

fn script_link_error(err: LinkError) -> ScriptError {
    ScriptError::compile_reason(err.code())
}

fn script_verify_error(err: VerifyError) -> ScriptError {
    ScriptError::compile_reason(err.code())
}

impl LanguageFrontend {
    /// 制品与诊断用的稳定前端标签（`valkyrie` / `lua` / `ruby`）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Valkyrie => "valkyrie",
            Self::Lua => "lua",
            Self::Ruby => "ruby",
        }
    }
}
