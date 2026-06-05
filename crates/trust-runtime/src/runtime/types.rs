//! Runtime shared types.

#![allow(missing_docs)]

use indexmap::IndexMap;
use smol_str::SmolStr;

use crate::value::Value;

pub(super) use trust_runtime_core::cycle::ReadyTask;
pub(super) use trust_runtime_core::retain::{RestartMode, RetainPolicy};

/// Source location of the most recently failed assertion during VM execution.
///
/// Populated by the bytecode interpreter when an `ASSERT_*` standard function
/// returns [`crate::error::RuntimeError::AssertionFailed`], so callers (such as
/// the ST test runner) can report the exact failing statement rather than the
/// enclosing POU declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionLocation {
    /// Source file label as recorded in the bytecode debug map.
    pub file: SmolStr,
    /// 1-based line of the failing assertion.
    pub line: u32,
    /// 1-based column of the failing assertion.
    pub column: u32,
}

/// Snapshot of retained global values for hot reload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RetainSnapshot {
    pub(crate) inner: trust_runtime_core::retain::RetainSnapshot,
}

impl RetainSnapshot {
    pub(crate) fn from_values(values: IndexMap<SmolStr, Value>) -> Self {
        Self {
            inner: trust_runtime_core::retain::RetainSnapshot::from_values(values),
        }
    }

    pub fn insert(&mut self, name: impl Into<SmolStr>, value: Value) {
        self.inner.insert(name, value);
    }

    #[must_use]
    pub fn values(&self) -> &IndexMap<SmolStr, Value> {
        self.inner.values()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GlobalInitValue {
    Value(Value),
    FunctionBlock { type_name: SmolStr },
    Class { type_name: SmolStr },
}

#[derive(Debug, Clone)]
pub(crate) struct GlobalVarMeta {
    // Preserved for upcoming diagnostics/profiling surfaces that report declared type metadata.
    #[allow(dead_code)]
    pub type_id: trust_hir::TypeId,
    pub retain: RetainPolicy,
    pub init: GlobalInitValue,
}
