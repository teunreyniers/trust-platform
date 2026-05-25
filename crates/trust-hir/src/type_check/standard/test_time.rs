use super::super::*;
use super::helpers::builtin_param;

impl<'a, 'b> StandardChecker<'a, 'b> {
    pub(in crate::type_check) fn infer_advance_time_call(&mut self, node: &SyntaxNode) -> TypeId {
        self.infer_test_time_call(node, "ADVANCE_TIME", "DT")
    }

    pub(in crate::type_check) fn infer_set_time_call(&mut self, node: &SyntaxNode) -> TypeId {
        self.infer_test_time_call(node, "SET_TIME", "T")
    }

    fn infer_test_time_call(&mut self, node: &SyntaxNode, name: &str, param: &str) -> TypeId {
        let params = vec![builtin_param(param, ParamDirection::In)];
        let call = self.builtin_call(node, params);
        call.check_formal_arg_count(self, node, 1);
        if call.arg_count() != 1 {
            return self
                .checker
                .legacy_suppressed_type(DiagnosticCode::WrongArgumentCount, node.text_range());
        }
        let Some((arg, arg_type)) = call.arg(0) else {
            return self
                .checker
                .legacy_suppressed_type(DiagnosticCode::WrongArgumentCount, node.text_range());
        };
        if !self.checker.is_assignable(TypeId::TIME, arg_type) {
            return self.checker.legacy_diagnostic_type(
                DiagnosticCode::InvalidArgumentType,
                arg.range,
                format!("{name} expects TIME input"),
            );
        }
        TypeId::VOID
    }
}
