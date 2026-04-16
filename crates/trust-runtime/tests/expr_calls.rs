use trust_runtime::harness::TestHarness;

#[test]
fn function_call_expr() {
    let source = r#"
        FUNCTION Add : INT
        VAR_INPUT
            a : INT;
            b : INT;
        END_VAR
        Add := a + b;
        END_FUNCTION

        PROGRAM Test
        VAR
            res : INT := 0;
        END_VAR
        res := Add(INT#2, INT#3);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("res", 5i16);
}

#[test]
fn function_call_named_args() {
    let source = r#"
        FUNCTION Add : INT
        VAR_INPUT
            a : INT;
            b : INT;
        END_VAR
        Add := a + b;
        END_FUNCTION

        PROGRAM Test
        VAR
            res : INT := 0;
        END_VAR
        res := Add(b := INT#2, a := INT#3);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("res", 5i16);
}

#[test]
fn function_call_output_positional() {
    let source = r#"
        FUNCTION WithOut : INT
        VAR_INPUT
            a : INT;
        END_VAR
        VAR_OUTPUT
            out1 : INT;
        END_VAR
        out1 := a;
        WithOut := out1;
        END_FUNCTION

        PROGRAM Test
        VAR
            a : INT := INT#4;
            out1 : INT := 0;
        END_VAR
        WithOut(a, out1);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("out1", 4i16);
}

#[test]
fn function_block_call() {
    let source = r#"
        FUNCTION_BLOCK Counter
        VAR_INPUT
            inc : BOOL;
        END_VAR
        VAR_OUTPUT
            value : INT;
        END_VAR
        VAR
            count : INT := INT#0;
        END_VAR
        IF inc THEN
            count := count + INT#1;
        END_IF;
        value := count;
        END_FUNCTION_BLOCK

        PROGRAM Test
        VAR
            fb : Counter;
            out : INT := 0;
        END_VAR
        fb(inc := TRUE, value => out);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("out", 1i16);
}

#[test]
fn stdlib_named_args() {
    let source = r#"
        PROGRAM Test
        VAR
            out : INT := 0;
        END_VAR
        out := SEL(G := TRUE, IN0 := INT#4, IN1 := INT#7);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("out", 7i16);
}

#[test]
fn function_in_out_with_conversion_expression_regression_issue_13() {
    let source = r#"
        FUNCTION incer2 : VOID
        VAR_IN_OUT
            i : USINT;
        END_VAR
        VAR_INPUT
            inc : USINT;
            dec : UINT;
        END_VAR
        i := i + inc - uint_to_usint(dec);
        END_FUNCTION

        PROGRAM Main
        VAR
            i : USINT := 100;
            j : UINT := 100;
            countDown : BOOL := FALSE;
        END_VAR

        IF i > 200 THEN
            countDown := TRUE;
        ELSIF i < 10 THEN
            countDown := FALSE;
        END_IF

        j := usint_to_uint(i);
        i := uint_to_usint(j);

        IF countDown THEN
            incer2(i, 0, 4);
        ELSE
            incer2(i, 4, 0);
        END_IF
        END_PROGRAM
    "#;

    let err = TestHarness::from_source(source)
        .err()
        .expect("generic in-out lowering should fail under current VM constraints");
    assert!(err.to_string().contains("unsupported generic type"));
}

#[test]
fn function_in_out_without_conversion_expression_baseline() {
    let source = r#"
        FUNCTION incer : VOID
        VAR_IN_OUT
            i : USINT;
        END_VAR
        VAR_INPUT
            inc : USINT;
            dec : USINT;
        END_VAR
        i := i + inc - dec;
        END_FUNCTION

        PROGRAM Main
        VAR
            i : USINT := 100;
            countDown : BOOL := FALSE;
        END_VAR

        IF countDown THEN
            incer(i, 0, 4);
        ELSE
            incer(i, 4, 0);
        END_IF
        END_PROGRAM
    "#;

    let err = TestHarness::from_source(source)
        .err()
        .expect("generic in-out lowering should fail under current VM constraints");
    assert!(err.to_string().contains("unsupported generic type"));
}

/// Single-file: a user-defined function whose name matches the `SRC_TO_DST`
/// conversion pattern must be called as a user function.
#[test]
fn user_fn_with_conversion_like_name_is_called_correctly() {
    let source = r#"
        FUNCTION UINT_TO_TIME : TIME
        VAR_INPUT
            IN : UDINT;
        END_VAR
        UINT_TO_TIME := MUL_TIME(IN1 := T#1ms, IN2 := IN);
        END_FUNCTION

        PROGRAM Main
        VAR
            t : TIME;
            x : UDINT := 500;
        END_VAR
        t := UINT_TO_TIME(x);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    let r1 = harness.cycle();
    assert!(
        r1.errors.is_empty(),
        "cycle 1 should produce no errors, got: {:?}",
        r1.errors
    );
    let r2 = harness.cycle();
    assert!(
        r2.errors.is_empty(),
        "cycle 2 should produce no errors, got: {:?}",
        r2.errors
    );
    harness.assert_eq(
        "t",
        trust_runtime::value::Value::Time(trust_runtime::value::Duration::from_millis(500)),
    );
}

/// Multi-file: the same scenario as above but with the function defined in a
/// separate source file, matching the real-world usage in otm_lib.
#[test]
fn user_fn_with_conversion_like_name_multifile_is_called_correctly() {
    let conversions = r#"
        FUNCTION UINT_TO_TIME : TIME
        VAR_INPUT
            IN : UDINT;
        END_VAR
        UINT_TO_TIME := MUL_TIME(IN1 := T#1ms, IN2 := IN);
        END_FUNCTION
    "#;

    let program = r#"
        PROGRAM Main
        VAR
            t : TIME;
            x : UDINT := 500;
        END_VAR
        t := UINT_TO_TIME(x);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_sources(&[conversions, program]).unwrap();
    let r1 = harness.cycle();
    assert!(
        r1.errors.is_empty(),
        "cycle 1 should produce no errors, got: {:?}",
        r1.errors
    );
    let r2 = harness.cycle();
    assert!(
        r2.errors.is_empty(),
        "cycle 2 should produce no errors, got: {:?}",
        r2.errors
    );
    harness.assert_eq(
        "t",
        trust_runtime::value::Value::Time(trust_runtime::value::Duration::from_millis(500)),
    );
}
