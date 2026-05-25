//! Test-only helpers for advancing the simulation clock in ST unit tests.

#![allow(missing_docs)]

use crate::error::RuntimeError;
use crate::value::{Duration, Value};

pub fn is_advance_time_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("ADVANCE_TIME")
}

pub fn is_set_time_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("SET_TIME")
}

pub fn is_test_time_helper_name(name: &str) -> bool {
    is_advance_time_name(name) || is_set_time_name(name)
}

pub fn parse_time_arg(value: &Value, name: &str) -> Result<Duration, RuntimeError> {
    match value {
        Value::Time(duration) => Ok(*duration),
        _ => Err(RuntimeError::ControlError(
            format!("{name} expects TIME input").into(),
        )),
    }
}
