use trust_runtime::harness::TestHarness;
use trust_runtime::value::Value;

const PROGRAM_STUB: &str = r#"
PROGRAM Main
END_PROGRAM
"#;

#[test]
fn byte_bit_access_in_program() {
    let source = r#"
PROGRAM Main
VAR
    mode : BYTE := BYTE#3;
    bit0 : BOOL;
END_VAR
bit0 := mode.0;
END_PROGRAM
"#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    assert_eq!(harness.get_output("bit0"), Some(Value::Bool(true)));
}

#[test]
fn byte_bit_access_in_function_block_body() {
    let source = format!(
        r#"
VAR_GLOBAL
    g_bit0 : BOOL;
END_VAR

FUNCTION_BLOCK ByteBitFb
VAR
    mode : BYTE := BYTE#3;
END_VAR
g_bit0 := mode.0;
END_FUNCTION_BLOCK
{PROGRAM_STUB}"#
    );

    let mut harness = TestHarness::from_source(&source).unwrap();
    harness
        .runtime_mut()
        .execute_function_block_by_name("ByteBitFb")
        .unwrap();
    assert_eq!(
        harness.runtime().storage().get_global("g_bit0"),
        Some(&Value::Bool(true))
    );
}

#[test]
fn byte_bit_access_via_nested_fb_call_from_program() {
    let source = format!(
        r#"
VAR_GLOBAL
    g_bit0 : BOOL;
END_VAR

FUNCTION_BLOCK ByteBitFb
VAR
    mode : BYTE := BYTE#3;
END_VAR
g_bit0 := mode.0;
END_FUNCTION_BLOCK

PROGRAM Main
VAR
    fb : ByteBitFb;
END_VAR
fb();
END_PROGRAM"#
    );

    let mut harness = TestHarness::from_source(&source).unwrap();
    let err = harness.cycle().errors.into_iter().next();
    assert!(err.is_none(), "expected success, got {err:?}");
    assert_eq!(
        harness.runtime().storage().get_global("g_bit0"),
        Some(&Value::Bool(true))
    );
}
