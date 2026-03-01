use std::time::Instant;

use smol_str::SmolStr;

use crate::bytecode::{TypeData, TypeTable};
use crate::debug::DebugHook;
use crate::error::RuntimeError;
use crate::eval::ops::{apply_binary, apply_unary, BinaryOp, UnaryOp};
use crate::memory::{FrameId, InstanceId, MemoryLocation};
use crate::task::ProgramDef;
use crate::value::{size_of_value, RefSegment, SizeOfError, Value, ValueRef};

use super::super::core::Runtime;
use super::call::{execute_native_call, push_call_frame, VM_LOCAL_SENTINEL_FRAME_ID};
use super::errors::VmTrap;
use super::frames::{FrameStack, VmFrame};
use super::stack::OperandStack;
use super::{VmModule, VmRef};

pub(super) fn execute_program(
    runtime: &mut Runtime,
    program: &ProgramDef,
) -> Result<(), RuntimeError> {
    let module = runtime.vm_module.clone().ok_or_else(|| {
        RuntimeError::InvalidConfig(
            "runtime.execution_backend='vm' requires loaded bytecode module".into(),
        )
    })?;

    let key = SmolStr::new(program.name.to_ascii_uppercase());
    let pou_id = module
        .program_ids
        .get(&key)
        .copied()
        .ok_or_else(|| VmTrap::MissingProgram(program.name.clone()).into_runtime_error())?;

    let instance_id = match runtime.storage.get_global(program.name.as_ref()) {
        Some(Value::Instance(id)) => Some(*id),
        _ => None,
    };

    execute_pou(runtime, module.as_ref(), pou_id, instance_id)
}

pub(super) fn execute_function_block_ref(
    runtime: &mut Runtime,
    reference: &ValueRef,
) -> Result<(), RuntimeError> {
    let module = runtime.vm_module.clone().ok_or_else(|| {
        RuntimeError::InvalidConfig(
            "runtime.execution_backend='vm' requires loaded bytecode module".into(),
        )
    })?;

    let instance_id = match runtime.storage.read_by_ref(reference.clone()) {
        Some(Value::Instance(id)) => *id,
        Some(_) => return Err(RuntimeError::TypeMismatch),
        None => return Err(RuntimeError::NullReference),
    };

    let instance = runtime
        .storage
        .get_instance(instance_id)
        .ok_or(RuntimeError::NullReference)?;
    let key = SmolStr::new(instance.type_name.to_ascii_uppercase());
    let pou_id = module
        .function_block_ids
        .get(&key)
        .copied()
        .ok_or_else(|| {
            VmTrap::MissingFunctionBlock(instance.type_name.clone()).into_runtime_error()
        })?;

    execute_pou(runtime, module.as_ref(), pou_id, Some(instance_id))
}

fn execute_pou(
    runtime: &mut Runtime,
    module: &VmModule,
    pou_id: u32,
    entry_instance: Option<InstanceId>,
) -> Result<(), RuntimeError> {
    let mut operand_stack = OperandStack::default();
    let mut frames = FrameStack::default();
    let mut pc = push_call_frame(&mut frames, module, pou_id, usize::MAX, entry_instance)
        .map_err(VmTrap::into_runtime_error)?;
    let mut budget = module.instruction_budget;

    loop {
        if frames.is_empty() {
            break;
        }

        let (frame_pou_id, frame_start, frame_end) = {
            let frame = frames
                .current()
                .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
            (frame.pou_id, frame.code_start, frame.code_end)
        };

        if pc == frame_end {
            let finished = frames.pop().map_err(VmTrap::into_runtime_error)?;
            if frames.is_empty() {
                break;
            }
            pc = finished.return_pc;
            continue;
        }

        if pc < frame_start || pc > frame_end {
            return Err(VmTrap::InvalidJumpTarget(pc as i64).into_runtime_error());
        }

        if budget == 0 {
            return Err(VmTrap::BudgetExceeded.into_runtime_error());
        }
        budget = budget.saturating_sub(1);

        if deadline_exceeded(runtime.execution_deadline) {
            return Err(VmTrap::DeadlineExceeded.into_runtime_error());
        }

        if let Some(location) = vm_statement_location(runtime, module, frame_pou_id, pc) {
            if let Some(debug) = runtime.debug.as_mut() {
                let call_depth = frames.len().saturating_sub(1) as u32;
                debug.on_statement(Some(&location), call_depth);
            }
        }

        let opcode = module
            .code
            .get(pc)
            .copied()
            .ok_or_else(|| VmTrap::BytecodeDecode("vm instruction fetch out of bounds".into()))
            .map_err(VmTrap::into_runtime_error)?;
        pc += 1;

        match opcode {
            0x00 => {}
            0x01 => return Err(VmTrap::ForStepZero.into_runtime_error()),
            0x02 => {
                let offset = read_i32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let frame = frames
                    .current()
                    .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
                apply_jump(&mut pc, offset, frame).map_err(VmTrap::into_runtime_error)?;
            }
            0x03 | 0x04 => {
                let offset = read_i32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let condition = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
                let condition = match condition {
                    Value::Bool(value) => value,
                    _ => return Err(VmTrap::ConditionNotBool.into_runtime_error()),
                };
                let should_jump = (opcode == 0x03 && condition) || (opcode == 0x04 && !condition);
                if should_jump {
                    let frame = frames
                        .current()
                        .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
                    apply_jump(&mut pc, offset, frame).map_err(VmTrap::into_runtime_error)?;
                }
            }
            0x05 => {
                let callee = read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let inherited_instance = frames.current().and_then(|frame| frame.runtime_instance);
                let return_pc = pc;
                pc = push_call_frame(&mut frames, module, callee, return_pc, inherited_instance)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x06 => {
                let finished = frames.pop().map_err(VmTrap::into_runtime_error)?;
                if frames.is_empty() {
                    break;
                }
                pc = finished.return_pc;
            }
            0x07 => return Err(VmTrap::UnsupportedOpcode("CALL_METHOD").into_runtime_error()),
            0x08 => return Err(VmTrap::UnsupportedOpcode("CALL_VIRTUAL").into_runtime_error()),
            0x09 => {
                let kind = read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let symbol_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let arg_count =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let frame = frames
                    .current_mut()
                    .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
                let result = execute_native_call(
                    runtime,
                    module,
                    frame,
                    &mut operand_stack,
                    kind,
                    symbol_idx,
                    arg_count,
                )
                .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(result)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x10 => {
                let const_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let value = module
                    .consts
                    .get(const_idx as usize)
                    .cloned()
                    .ok_or(VmTrap::InvalidConstIndex(const_idx))
                    .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(value)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x11 => operand_stack
                .duplicate_top()
                .map_err(VmTrap::into_runtime_error)?,
            0x12 => {
                let _ = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
            }
            0x13 => operand_stack
                .swap_top()
                .map_err(VmTrap::into_runtime_error)?,
            0x14 => return Err(VmTrap::UnsupportedOpcode("ROT3").into_runtime_error()),
            0x15 => return Err(VmTrap::UnsupportedOpcode("ROT4").into_runtime_error()),
            0x16 => return Err(VmTrap::UnsupportedOpcode("CAST_IMPLICIT").into_runtime_error()),
            0x20 => {
                let ref_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let value = load_ref(runtime, module, &frames, ref_idx)
                    .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(value)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x21 => {
                let ref_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let value = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
                store_ref(runtime, module, &mut frames, ref_idx, value)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x22 => {
                let ref_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let value_ref =
                    load_ref_addr(module, &frames, ref_idx).map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(Value::Reference(Some(value_ref)))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x23 => {
                let frame = frames
                    .current()
                    .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
                let self_instance = frame.runtime_instance.ok_or_else(|| {
                    VmTrap::Runtime(RuntimeError::TypeMismatch).into_runtime_error()
                })?;
                operand_stack
                    .push(Value::Instance(self_instance))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x24 => {
                let frame = frames
                    .current()
                    .ok_or_else(|| VmTrap::CallStackUnderflow.into_runtime_error())?;
                let self_instance = frame.runtime_instance.ok_or_else(|| {
                    VmTrap::Runtime(RuntimeError::TypeMismatch).into_runtime_error()
                })?;
                let instance = runtime
                    .storage
                    .get_instance(self_instance)
                    .ok_or_else(|| VmTrap::NullReference.into_runtime_error())?;
                let super_instance = instance.parent.ok_or_else(|| {
                    VmTrap::Runtime(RuntimeError::TypeMismatch).into_runtime_error()
                })?;
                operand_stack
                    .push(Value::Instance(super_instance))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x30 => {
                let field_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let field = module
                    .strings
                    .get(field_idx as usize)
                    .cloned()
                    .ok_or_else(|| {
                        VmTrap::BytecodeDecode(
                            format!("invalid index {field_idx} for string").into(),
                        )
                        .into_runtime_error()
                    })?;
                let reference =
                    pop_reference(&mut operand_stack).map_err(VmTrap::into_runtime_error)?;
                let next = dynamic_ref_field(runtime, &frames, reference, field)
                    .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(Value::Reference(Some(next)))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x31 => {
                let index = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
                let index = index_to_i64(index).map_err(VmTrap::into_runtime_error)?;
                let reference =
                    pop_reference(&mut operand_stack).map_err(VmTrap::into_runtime_error)?;
                let next = dynamic_ref_index(runtime, &frames, reference, index)
                    .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(Value::Reference(Some(next)))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x32 => {
                let reference =
                    pop_reference(&mut operand_stack).map_err(VmTrap::into_runtime_error)?;
                let value = dynamic_load_ref(runtime, &frames, &reference)
                    .map_err(VmTrap::into_runtime_error)?;
                operand_stack
                    .push(value)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x33 => {
                let value = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
                let reference =
                    pop_reference(&mut operand_stack).map_err(VmTrap::into_runtime_error)?;
                dynamic_store_ref(runtime, &mut frames, &reference, value)
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x40 => execute_binary(runtime, &mut operand_stack, BinaryOp::Add)
                .map_err(VmTrap::into_runtime_error)?,
            0x41 => execute_binary(runtime, &mut operand_stack, BinaryOp::Sub)
                .map_err(VmTrap::into_runtime_error)?,
            0x42 => execute_binary(runtime, &mut operand_stack, BinaryOp::Mul)
                .map_err(VmTrap::into_runtime_error)?,
            0x43 => execute_binary(runtime, &mut operand_stack, BinaryOp::Div)
                .map_err(VmTrap::into_runtime_error)?,
            0x44 => execute_binary(runtime, &mut operand_stack, BinaryOp::Mod)
                .map_err(VmTrap::into_runtime_error)?,
            0x45 => execute_unary(&mut operand_stack, UnaryOp::Neg)
                .map_err(VmTrap::into_runtime_error)?,
            0x46 => execute_binary(runtime, &mut operand_stack, BinaryOp::And)
                .map_err(VmTrap::into_runtime_error)?,
            0x47 => execute_binary(runtime, &mut operand_stack, BinaryOp::Or)
                .map_err(VmTrap::into_runtime_error)?,
            0x48 => execute_binary(runtime, &mut operand_stack, BinaryOp::Xor)
                .map_err(VmTrap::into_runtime_error)?,
            0x49 => execute_unary(&mut operand_stack, UnaryOp::Not)
                .map_err(VmTrap::into_runtime_error)?,
            0x4A => return Err(VmTrap::UnsupportedOpcode("SHL").into_runtime_error()),
            0x4B => return Err(VmTrap::UnsupportedOpcode("SHR").into_runtime_error()),
            0x4C => execute_binary(runtime, &mut operand_stack, BinaryOp::Pow)
                .map_err(VmTrap::into_runtime_error)?,
            0x4D => return Err(VmTrap::UnsupportedOpcode("ROL").into_runtime_error()),
            0x4E => return Err(VmTrap::UnsupportedOpcode("ROR").into_runtime_error()),
            0x50 => execute_binary(runtime, &mut operand_stack, BinaryOp::Eq)
                .map_err(VmTrap::into_runtime_error)?,
            0x51 => execute_binary(runtime, &mut operand_stack, BinaryOp::Ne)
                .map_err(VmTrap::into_runtime_error)?,
            0x52 => execute_binary(runtime, &mut operand_stack, BinaryOp::Lt)
                .map_err(VmTrap::into_runtime_error)?,
            0x53 => execute_binary(runtime, &mut operand_stack, BinaryOp::Le)
                .map_err(VmTrap::into_runtime_error)?,
            0x54 => execute_binary(runtime, &mut operand_stack, BinaryOp::Gt)
                .map_err(VmTrap::into_runtime_error)?,
            0x55 => execute_binary(runtime, &mut operand_stack, BinaryOp::Ge)
                .map_err(VmTrap::into_runtime_error)?,
            0x60 => {
                let type_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
                let size = sizeof_type_from_table(&module.types, type_idx)
                    .map_err(|err| VmTrap::Runtime(err).into_runtime_error())?;
                let size = i32::try_from(size)
                    .map_err(|_| VmTrap::Runtime(RuntimeError::Overflow).into_runtime_error())?;
                operand_stack
                    .push(Value::DInt(size))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x61 => {
                let value = operand_stack.pop().map_err(VmTrap::into_runtime_error)?;
                let size = size_of_value(runtime.registry(), &value)
                    .map_err(sizeof_error_to_runtime)
                    .map_err(|err| VmTrap::Runtime(err).into_runtime_error())?;
                let size = i32::try_from(size)
                    .map_err(|_| VmTrap::Runtime(RuntimeError::Overflow).into_runtime_error())?;
                operand_stack
                    .push(Value::DInt(size))
                    .map_err(VmTrap::into_runtime_error)?;
            }
            0x70 => {
                let _debug_idx =
                    read_u32(&module.code, &mut pc).map_err(VmTrap::into_runtime_error)?;
            }
            _ => return Err(VmTrap::InvalidOpcode(opcode).into_runtime_error()),
        }
    }

    Ok(())
}

fn deadline_exceeded(deadline: Option<Instant>) -> bool {
    match deadline {
        Some(deadline) => Instant::now() >= deadline,
        None => false,
    }
}

fn vm_statement_location(
    runtime: &Runtime,
    module: &VmModule,
    pou_id: u32,
    pc: usize,
) -> Option<crate::debug::SourceLocation> {
    let source = module.debug_map.source_by_pc.get(&(pou_id, pc as u32))?;
    runtime.resolve_vm_debug_location(source.file.as_str(), source.line, source.column)
}

fn execute_unary(stack: &mut OperandStack, op: UnaryOp) -> Result<(), VmTrap> {
    let value = stack.pop()?;
    let result = apply_unary(op, value)?;
    stack.push(result)
}

fn execute_binary(runtime: &Runtime, stack: &mut OperandStack, op: BinaryOp) -> Result<(), VmTrap> {
    let (left, right) = stack.pop_pair()?;
    let result = apply_binary(op, left, right, &runtime.profile)?;
    stack.push(result)
}

fn load_ref(
    runtime: &Runtime,
    module: &VmModule,
    frames: &FrameStack,
    ref_idx: u32,
) -> Result<Value, VmTrap> {
    let reference = module
        .refs
        .get(ref_idx as usize)
        .ok_or(VmTrap::InvalidRefIndex(ref_idx))?;

    match reference {
        VmRef::Local { path, .. } => {
            if !path.is_empty() {
                return Err(VmTrap::UnsupportedRefLocation("local-path"));
            }
            let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
            frame.load_local(ref_idx)
        }
        _ => {
            let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
            let (location, offset, path) = runtime_access_target(reference, frame)?;
            runtime
                .storage
                .read_by_ref_parts(location, offset, path)
                .cloned()
                .ok_or(VmTrap::NullReference)
        }
    }
}

fn load_ref_addr(module: &VmModule, frames: &FrameStack, ref_idx: u32) -> Result<ValueRef, VmTrap> {
    let reference = module
        .refs
        .get(ref_idx as usize)
        .ok_or(VmTrap::InvalidRefIndex(ref_idx))?;
    match reference {
        VmRef::Local { path, .. } => {
            if !path.is_empty() {
                return Err(VmTrap::UnsupportedRefLocation("local-path"));
            }
            let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
            let local_slot = frame.local_slot_index(ref_idx)?;
            Ok(ValueRef {
                location: MemoryLocation::Local(FrameId(VM_LOCAL_SENTINEL_FRAME_ID)),
                offset: local_slot,
                path: Vec::new(),
            })
        }
        _ => {
            let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
            let (location, offset, path) = runtime_access_target(reference, frame)?;
            Ok(ValueRef {
                location,
                offset,
                path: path.to_vec(),
            })
        }
    }
}

fn store_ref(
    runtime: &mut Runtime,
    module: &VmModule,
    frames: &mut FrameStack,
    ref_idx: u32,
    value: Value,
) -> Result<(), VmTrap> {
    let reference = module
        .refs
        .get(ref_idx as usize)
        .ok_or(VmTrap::InvalidRefIndex(ref_idx))?;

    match reference {
        VmRef::Local { path, .. } => {
            if !path.is_empty() {
                return Err(VmTrap::UnsupportedRefLocation("local-path"));
            }
            let frame = frames.current_mut().ok_or(VmTrap::CallStackUnderflow)?;
            frame.store_local(ref_idx, value)
        }
        _ => {
            let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
            let (location, offset, path) = runtime_access_target(reference, frame)?;
            if runtime
                .storage
                .write_by_ref_parts(location, offset, path, value)
            {
                Ok(())
            } else {
                Err(VmTrap::NullReference)
            }
        }
    }
}

fn pop_reference(stack: &mut OperandStack) -> Result<ValueRef, VmTrap> {
    let value = stack.pop()?;
    match value {
        Value::Reference(Some(reference)) => Ok(reference),
        Value::Reference(None) => Err(VmTrap::NullReference),
        _ => Err(VmTrap::Runtime(RuntimeError::TypeMismatch)),
    }
}

fn dynamic_ref_field(
    runtime: &Runtime,
    frames: &FrameStack,
    mut reference: ValueRef,
    field: SmolStr,
) -> Result<ValueRef, VmTrap> {
    let target = dynamic_load_ref(runtime, frames, &reference)?;
    match target {
        Value::Struct(struct_value) => {
            if !struct_value.fields.contains_key(field.as_str()) {
                return Err(VmTrap::Runtime(RuntimeError::UndefinedField(field)));
            }
            reference.path.push(RefSegment::Field(field));
            Ok(reference)
        }
        Value::Instance(instance_id) => runtime
            .storage
            .ref_for_instance_recursive(instance_id, field.as_str())
            .ok_or(VmTrap::Runtime(RuntimeError::UndefinedField(field))),
        _ => Err(VmTrap::Runtime(RuntimeError::TypeMismatch)),
    }
}

fn dynamic_ref_index(
    runtime: &Runtime,
    frames: &FrameStack,
    mut reference: ValueRef,
    index: i64,
) -> Result<ValueRef, VmTrap> {
    let target = dynamic_load_ref(runtime, frames, &reference)?;
    match target {
        Value::Array(array) => {
            if array.dimensions.len() != 1 {
                return Err(VmTrap::Runtime(RuntimeError::TypeMismatch));
            }
            let (lower, upper) = array.dimensions[0];
            if index < lower || index > upper {
                return Err(VmTrap::Runtime(RuntimeError::IndexOutOfBounds {
                    index,
                    lower,
                    upper,
                }));
            }
            reference.path.push(RefSegment::Index(vec![index]));
            Ok(reference)
        }
        _ => Err(VmTrap::Runtime(RuntimeError::TypeMismatch)),
    }
}

fn dynamic_load_ref(
    runtime: &Runtime,
    frames: &FrameStack,
    reference: &ValueRef,
) -> Result<Value, VmTrap> {
    if matches!(
        reference.location,
        MemoryLocation::Local(FrameId(VM_LOCAL_SENTINEL_FRAME_ID))
    ) {
        let frame = frames.current().ok_or(VmTrap::CallStackUnderflow)?;
        return read_vm_local_ref(frame, reference.offset, &reference.path);
    }
    runtime
        .storage
        .read_by_ref(reference.clone())
        .cloned()
        .ok_or(VmTrap::NullReference)
}

fn dynamic_store_ref(
    runtime: &mut Runtime,
    frames: &mut FrameStack,
    reference: &ValueRef,
    value: Value,
) -> Result<(), VmTrap> {
    if matches!(
        reference.location,
        MemoryLocation::Local(FrameId(VM_LOCAL_SENTINEL_FRAME_ID))
    ) {
        let frame = frames.current_mut().ok_or(VmTrap::CallStackUnderflow)?;
        return write_vm_local_ref(frame, reference.offset, &reference.path, value);
    }
    if runtime.storage.write_by_ref(reference.clone(), value) {
        Ok(())
    } else {
        Err(VmTrap::NullReference)
    }
}

fn read_vm_local_ref(frame: &VmFrame, offset: usize, path: &[RefSegment]) -> Result<Value, VmTrap> {
    let root = frame.locals.get(offset).ok_or(VmTrap::NullReference)?;
    read_value_path(root, path)
        .cloned()
        .ok_or(VmTrap::NullReference)
}

fn write_vm_local_ref(
    frame: &mut VmFrame,
    offset: usize,
    path: &[RefSegment],
    value: Value,
) -> Result<(), VmTrap> {
    let root = frame.locals.get_mut(offset).ok_or(VmTrap::NullReference)?;
    if write_value_path(root, path, value) {
        Ok(())
    } else {
        Err(VmTrap::NullReference)
    }
}

fn read_value_path<'a>(value: &'a Value, path: &[RefSegment]) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(value);
    }
    match &path[0] {
        RefSegment::Field(name) => match value {
            Value::Struct(struct_value) => struct_value
                .fields
                .get(name)
                .and_then(|field| read_value_path(field, &path[1..])),
            _ => None,
        },
        RefSegment::Index(indices) => match value {
            Value::Array(array) => {
                let offset = array_offset_i64(&array.dimensions, indices)?;
                array
                    .elements
                    .get(offset)
                    .and_then(|element| read_value_path(element, &path[1..]))
            }
            _ => None,
        },
    }
}

fn write_value_path(target: &mut Value, path: &[RefSegment], value: Value) -> bool {
    if path.is_empty() {
        *target = value;
        return true;
    }

    match &path[0] {
        RefSegment::Field(name) => match target {
            Value::Struct(struct_value) => struct_value
                .fields
                .get_mut(name)
                .map(|field| write_value_path(field, &path[1..], value))
                .unwrap_or(false),
            _ => false,
        },
        RefSegment::Index(indices) => match target {
            Value::Array(array) => {
                let offset = match array_offset_i64(&array.dimensions, indices) {
                    Some(offset) => offset,
                    None => return false,
                };
                array
                    .elements
                    .get_mut(offset)
                    .map(|element| write_value_path(element, &path[1..], value))
                    .unwrap_or(false)
            }
            _ => false,
        },
    }
}

fn array_offset_i64(dimensions: &[(i64, i64)], indices: &[i64]) -> Option<usize> {
    if dimensions.len() != indices.len() {
        return None;
    }
    let mut offset: i128 = 0;
    let mut stride: i128 = 1;
    for ((lower, upper), index) in dimensions.iter().zip(indices).rev() {
        if index < lower || index > upper {
            return None;
        }
        let len = (*upper - *lower + 1) as i128;
        offset += (index - *lower) as i128 * stride;
        stride *= len;
    }
    usize::try_from(offset).ok()
}

fn index_to_i64(value: Value) -> Result<i64, VmTrap> {
    match value {
        Value::SInt(v) => Ok(v as i64),
        Value::Int(v) => Ok(v as i64),
        Value::DInt(v) => Ok(v as i64),
        Value::LInt(v) => Ok(v),
        Value::USInt(v) => Ok(v as i64),
        Value::UInt(v) => Ok(v as i64),
        Value::UDInt(v) => Ok(v as i64),
        Value::ULInt(v) => {
            i64::try_from(v).map_err(|_| VmTrap::Runtime(RuntimeError::TypeMismatch))
        }
        _ => Err(VmTrap::Runtime(RuntimeError::TypeMismatch)),
    }
}

fn sizeof_error_to_runtime(err: SizeOfError) -> RuntimeError {
    match err {
        SizeOfError::Overflow => RuntimeError::Overflow,
        SizeOfError::UnknownType | SizeOfError::UnsupportedType => RuntimeError::TypeMismatch,
    }
}

fn sizeof_type_from_table(types: &TypeTable, type_idx: u32) -> Result<u64, RuntimeError> {
    let mut stack = Vec::new();
    sizeof_type_from_table_inner(types, type_idx, &mut stack)
}

const SIZEOF_TYPE_MAX_DEPTH: usize = 128;

fn sizeof_type_from_table_inner(
    types: &TypeTable,
    type_idx: u32,
    stack: &mut Vec<u32>,
) -> Result<u64, RuntimeError> {
    if stack.len() >= SIZEOF_TYPE_MAX_DEPTH {
        return Err(RuntimeError::InvalidBytecode(
            format!(
                "SIZEOF type nesting exceeds max depth {SIZEOF_TYPE_MAX_DEPTH} at index {type_idx}"
            )
            .into(),
        ));
    }
    if stack.contains(&type_idx) {
        return Err(RuntimeError::InvalidBytecode(
            format!("SIZEOF type recursion at index {type_idx}").into(),
        ));
    }
    let entry = types.entries.get(type_idx as usize).ok_or_else(|| {
        RuntimeError::InvalidBytecode(format!("invalid type index {type_idx} for SIZEOF").into())
    })?;
    stack.push(type_idx);
    let result = (|| match &entry.data {
        TypeData::Primitive {
            prim_id,
            max_length,
        } => sizeof_primitive_type(*prim_id, *max_length),
        TypeData::Array { elem_type_id, dims } => {
            let elem_size = sizeof_type_from_table_inner(types, *elem_type_id, stack)?;
            let len = type_array_len(dims).ok_or(RuntimeError::TypeMismatch)?;
            elem_size.checked_mul(len).ok_or(RuntimeError::Overflow)
        }
        TypeData::Struct { fields } => {
            let mut total = 0u64;
            for field in fields {
                let size = sizeof_type_from_table_inner(types, field.type_id, stack)?;
                total = total.checked_add(size).ok_or(RuntimeError::Overflow)?;
            }
            Ok(total)
        }
        TypeData::Union { fields } => {
            let mut max_size = 0u64;
            for field in fields {
                let size = sizeof_type_from_table_inner(types, field.type_id, stack)?;
                max_size = max_size.max(size);
            }
            Ok(max_size)
        }
        TypeData::Enum { base_type_id, .. } => {
            sizeof_type_from_table_inner(types, *base_type_id, stack)
        }
        TypeData::Alias { target_type_id } => {
            sizeof_type_from_table_inner(types, *target_type_id, stack)
        }
        TypeData::Subrange { base_type_id, .. } => {
            sizeof_type_from_table_inner(types, *base_type_id, stack)
        }
        TypeData::Reference { .. } => {
            u64::try_from(std::mem::size_of::<ValueRef>()).map_err(|_| RuntimeError::Overflow)
        }
        TypeData::Pou { .. } | TypeData::Interface { .. } => Err(RuntimeError::TypeMismatch),
    })();
    let _ = stack.pop();
    result
}

fn sizeof_primitive_type(prim_id: u16, max_length: u16) -> Result<u64, RuntimeError> {
    match prim_id {
        1 | 2 | 6 | 10 | 26 => Ok(1),
        3 | 7 | 11 | 27 => Ok(2),
        4 | 8 | 12 | 14 => Ok(4),
        5 | 9 | 13 | 15 => Ok(8),
        16 | 18 | 20 | 22 => Ok(4),
        17 | 19 | 21 | 23 => Ok(8),
        24 => {
            if max_length == 0 {
                Err(RuntimeError::TypeMismatch)
            } else {
                Ok(u64::from(max_length))
            }
        }
        25 => {
            if max_length == 0 {
                Err(RuntimeError::TypeMismatch)
            } else {
                u64::from(max_length)
                    .checked_mul(2)
                    .ok_or(RuntimeError::Overflow)
            }
        }
        other => Err(RuntimeError::InvalidBytecode(
            format!("unsupported primitive type id {other} for SIZEOF").into(),
        )),
    }
}

fn type_array_len(dimensions: &[(i64, i64)]) -> Option<u64> {
    let mut total: i128 = 1;
    for (lower, upper) in dimensions {
        let len = i128::from(*upper) - i128::from(*lower) + 1;
        if len <= 0 {
            return None;
        }
        total = total.checked_mul(len)?;
    }
    u64::try_from(total).ok()
}

fn runtime_access_target<'a>(
    reference: &'a VmRef,
    frame: &VmFrame,
) -> Result<(MemoryLocation, usize, &'a [crate::value::RefSegment]), VmTrap> {
    match reference {
        VmRef::Global { offset, path } => Ok((MemoryLocation::Global, *offset, path.as_slice())),
        VmRef::Instance {
            owner_instance_id,
            offset,
            path,
        } => {
            let runtime_owner = if frame.instance_owner == Some(*owner_instance_id) {
                frame
                    .runtime_instance
                    .unwrap_or(InstanceId(*owner_instance_id))
            } else {
                InstanceId(*owner_instance_id)
            };
            Ok((
                MemoryLocation::Instance(runtime_owner),
                *offset,
                path.as_slice(),
            ))
        }
        VmRef::Local {
            owner_frame_id,
            offset,
            path,
        } => Err(VmTrap::UnsupportedRefLocation(if path.is_empty() {
            let _ = owner_frame_id;
            let _ = offset;
            "local"
        } else {
            "local-path"
        })),
        VmRef::Retain { offset, path } => {
            let _ = (offset, path);
            Err(VmTrap::UnsupportedRefLocation("retain"))
        }
        VmRef::Io { area, offset, path } => {
            let _ = (area, offset, path);
            Err(VmTrap::UnsupportedRefLocation("io"))
        }
    }
}

fn apply_jump(pc: &mut usize, offset: i32, frame: &VmFrame) -> Result<(), VmTrap> {
    let base = *pc as i64;
    let target = base + i64::from(offset);
    if target < frame.code_start as i64 || target > frame.code_end as i64 {
        return Err(VmTrap::InvalidJumpTarget(target));
    }
    *pc = target as usize;
    Ok(())
}

fn read_u32(code: &[u8], pc: &mut usize) -> Result<u32, VmTrap> {
    if *pc + 4 > code.len() {
        return Err(VmTrap::BytecodeDecode(
            "vm operand read overflow (u32)".into(),
        ));
    }
    let bytes = [code[*pc], code[*pc + 1], code[*pc + 2], code[*pc + 3]];
    *pc += 4;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32(code: &[u8], pc: &mut usize) -> Result<i32, VmTrap> {
    if *pc + 4 > code.len() {
        return Err(VmTrap::BytecodeDecode(
            "vm operand read overflow (i32)".into(),
        ));
    }
    let bytes = [code[*pc], code[*pc + 1], code[*pc + 2], code[*pc + 3]];
    *pc += 4;
    Ok(i32::from_le_bytes(bytes))
}
