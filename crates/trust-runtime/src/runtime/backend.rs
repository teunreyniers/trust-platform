//! Runtime execution backend dispatch seam.

use crate::error;
use crate::execution_backend::ExecutionBackend;
use crate::task::ProgramDef;
use crate::value::ValueRef;
use std::sync::Arc;

use super::core::Runtime;

/// Backend contract for runtime execution paths.
pub(super) trait RuntimeExecutionBackend {
    fn execute_program(
        &self,
        runtime: &mut Runtime,
        program: &ProgramDef,
    ) -> Result<(), error::RuntimeError>;

    fn execute_function_block_ref(
        &self,
        runtime: &mut Runtime,
        reference: &ValueRef,
    ) -> Result<(), error::RuntimeError>;
}

#[cfg(feature = "legacy-interpreter")]
struct InterpreterBackend;
struct BytecodeVmBackend;

#[cfg(feature = "legacy-interpreter")]
static INTERPRETER_BACKEND: InterpreterBackend = InterpreterBackend;
static BYTECODE_VM_BACKEND: BytecodeVmBackend = BytecodeVmBackend;

pub(super) fn resolve_backend(mode: ExecutionBackend) -> &'static dyn RuntimeExecutionBackend {
    match mode {
        #[cfg(feature = "legacy-interpreter")]
        ExecutionBackend::Interpreter => &INTERPRETER_BACKEND,
        ExecutionBackend::BytecodeVm => &BYTECODE_VM_BACKEND,
    }
}

pub(super) fn validate_backend_selection(
    _runtime: &Runtime,
    _mode: ExecutionBackend,
) -> Result<(), error::RuntimeError> {
    Ok(())
}

#[cfg(feature = "legacy-interpreter")]
impl RuntimeExecutionBackend for InterpreterBackend {
    fn execute_program(
        &self,
        runtime: &mut Runtime,
        program: &ProgramDef,
    ) -> Result<(), error::RuntimeError> {
        runtime.execute_program_interpreter(program)
    }

    fn execute_function_block_ref(
        &self,
        runtime: &mut Runtime,
        reference: &ValueRef,
    ) -> Result<(), error::RuntimeError> {
        runtime.execute_function_block_ref_interpreter(reference)
    }
}

impl RuntimeExecutionBackend for BytecodeVmBackend {
    fn execute_program(
        &self,
        runtime: &mut Runtime,
        program: &ProgramDef,
    ) -> Result<(), error::RuntimeError> {
        ensure_vm_module_loaded(runtime)?;
        super::vm::execute_program(runtime, program)
    }

    fn execute_function_block_ref(
        &self,
        runtime: &mut Runtime,
        reference: &ValueRef,
    ) -> Result<(), error::RuntimeError> {
        ensure_vm_module_loaded(runtime)?;
        super::vm::execute_function_block_ref(runtime, reference)
    }
}

fn ensure_vm_module_loaded(runtime: &mut Runtime) -> Result<(), error::RuntimeError> {
    if runtime.vm_module.is_some() {
        return Ok(());
    }
    let module = build_vm_module(runtime)?;
    module
        .validate()
        .map_err(|err| error::RuntimeError::InvalidBytecode(err.to_string().into()))?;
    let vm_module = Arc::new(super::vm::VmModule::from_bytecode(&module)?);
    runtime.vm_module = Some(vm_module);
    Ok(())
}

fn build_vm_module(
    runtime: &Runtime,
) -> Result<crate::bytecode::BytecodeModule, error::RuntimeError> {
    if runtime.source_text_index.is_empty() {
        return crate::bytecode::BytecodeModule::from_runtime(runtime).map_err(|err| {
            error::RuntimeError::InvalidBytecode(format!("vm module build failed: {err}").into())
        });
    }

    let max_file_id = runtime.source_text_index.keys().copied().max().unwrap_or(0);
    let sources = (0..=max_file_id)
        .map(|file_id| {
            runtime
                .source_text_index
                .get(&file_id)
                .map(String::as_str)
                .unwrap_or("")
        })
        .collect::<Vec<_>>();
    crate::bytecode::BytecodeModule::from_runtime_with_sources(runtime, &sources).map_err(|err| {
        error::RuntimeError::InvalidBytecode(format!("vm module build failed: {err}").into())
    })
}
