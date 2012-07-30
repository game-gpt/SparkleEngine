//! 脚本编译门面：产出制品，不持有 VM / JIT。

use crate::artifact::{
    ExecutableImage, LinkError, LinkedProgram, SparkObject, VerifyError,
};
use crate::host_schema::{HostFunction, HostFunctionId, HostSchema};
use crate::request::{CompilationRequest, LanguageFrontend};
use crate::{compile_module, compile_module_with_registry, ScriptError, ScriptLanguage};
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
}

impl ScriptCompiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 按正式 [`CompilationRequest`] 编译并链接、验证。
    pub fn compile(&mut self, request: &CompilationRequest) -> Result<CompiledPackage, ScriptError> {
        let source = request.primary_source().ok_or_else(|| {
            ScriptError::compile_reason("compilation_request_missing_source")
        })?;
        let language = ScriptLanguage::from(request.language.frontend);
        let module = match language {
            ScriptLanguage::Valkyrie => {
                let reg = request.host_schema.to_native_registry();
                spark_script_valkyrie::compile_with_registry(source, &reg)?
            }
            ScriptLanguage::Lua | ScriptLanguage::Ruby => {
                let names = request.host_schema.short_names();
                compile_module(language, source, &names)?
            }
        };
        self.seal(request, module)
    }

    /// 过渡期便利：语言 + 源码 + schema。
    pub fn compile_source(
        &mut self,
        language: ScriptLanguage,
        source: &str,
        host: &HostSchema,
    ) -> Result<CompiledPackage, ScriptError> {
        let request = CompilationRequest::repl(language, source, host.clone());
        self.compile(&request)
    }

    /// 过渡期：仅函数名列表（自动生成最小 schema 桩）。
    pub fn compile_with_native_names(
        &mut self,
        language: ScriptLanguage,
        source: &str,
        natives: &[&str],
    ) -> Result<CompiledPackage, ScriptError> {
        let host = stub_schema_from_names(natives);
        let module = compile_module(language, source, natives)?;
        let request = CompilationRequest::repl(language, source, host);
        self.seal(&request, module)
    }

    pub(crate) fn seal(
        &mut self,
        request: &CompilationRequest,
        module: spark_vm::Module,
    ) -> Result<CompiledPackage, ScriptError> {
        let object = SparkObject::from_legacy_module(
            request.package.clone(),
            request.language.clone(),
            &request.host_schema,
            module,
        );
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

/// 仅用 registry 编译（Valkyrie 走完整签名；其它前端取名）。
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
