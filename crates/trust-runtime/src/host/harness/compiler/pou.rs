use smol_str::SmolStr;
use trust_hir::db::FileId;
use trust_hir::semantic::{DeclarationCatalog, DeclarationKind};
use trust_hir::symbols::ParamDirection;
use trust_syntax::syntax::{SyntaxKind, SyntaxNode};

use crate::io::IoAddress;
use crate::program_model::{
    ClassDef, FunctionBlockBase, FunctionBlockDef, FunctionDef, InterfaceDef, MethodDef, Param,
    VarDef,
};
use crate::task::ProgramDef;

use super::super::lower::{lower_expr, lower_stmt_list, resolve_initializer_enum_variant};
use super::super::types::CompileError;
use super::super::util::{collect_using_directives, namespace_qualified_name, node_text};
use super::model::{GlobalInit, LoweredProgram, LoweringContext, LoweringInputs, ProgramVars};
use super::vars::{parse_var_decl, var_block_kind, var_block_qualifiers, VarBlockKind};
use super::{lower_type_ref, resolve_named_type};

include!("pou/entry_points.rs");
include!("pou/node_lowering.rs");
include!("pou/names.rs");
include!("pou/program_vars.rs");
include!("pou/function_vars.rs");
include!("pou/class_vars.rs");
include!("pou/function_block_vars.rs");

/// Registers the compile-time value of every `VAR CONSTANT` declared in `node`
/// before any variable types are lowered.
///
/// Variable declarations may reference constants in constant expressions (array
/// bounds, string lengths, etc.). Because the main lowering pass walks `VAR`
/// blocks in source order, a constant declared in a later block than the
/// declaration that uses it would otherwise be unresolved. Registering all
/// constants first makes the relative order of constant and non-constant blocks
/// irrelevant. Constants are evaluated in declaration order, so a constant may
/// reference another constant only if that one is declared earlier.
/// `is_constant_kind` selects which block kinds contribute constants for the
/// enclosing POU.
fn register_pou_compile_time_constants(
    node: &SyntaxNode,
    ctx: &mut LoweringContext<'_>,
    is_constant_kind: impl Fn(VarBlockKind) -> bool,
) -> Result<(), CompileError> {
    for var_block in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::VarBlock)
    {
        let kind = var_block_kind(&var_block)?;
        if !var_block_qualifiers(&var_block).constant || !is_constant_kind(kind) {
            continue;
        }
        for var_decl in var_block
            .children()
            .filter(|child| child.kind() == SyntaxKind::VarDecl)
        {
            let parts = parse_var_decl(&var_decl)?;
            let Some(init) = parts.initializer.as_ref() else {
                continue;
            };
            let type_id = lower_type_ref(&parts.type_ref, ctx)?;
            let lowered = lower_expr(init, ctx)?;
            let lowered = resolve_initializer_enum_variant(init, lowered, type_id, ctx)?;
            let value = ctx.eval_compile_time_const_initializer(&lowered, type_id)?;
            for name in &parts.names {
                ctx.register_compile_time_const(name.as_str(), value.clone());
                if matches!(kind, VarBlockKind::Global) {
                    let qualified = namespace_qualified_name(&var_block, name.as_str());
                    ctx.register_compile_time_const(qualified.as_str(), value.clone());
                }
            }
        }
    }
    Ok(())
}
