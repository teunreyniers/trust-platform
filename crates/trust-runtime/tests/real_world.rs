use trust_runtime::harness::TestHarness;

#[test]
fn samples() {
    // Vendor-neutral ST sample covering common constructs without vendor extensions.
    let source = r#"
        FUNCTION ClampReal : REAL
        VAR_INPUT
            Value : REAL;
            Min : REAL;
            Max : REAL;
        END_VAR
        IF Value < Min THEN
            ClampReal := Min;
        ELSIF Value > Max THEN
            ClampReal := Max;
        ELSE
            ClampReal := Value;
        END_IF
        END_FUNCTION

        FUNCTION Avg4 : REAL
        VAR_INPUT
            Samples : ARRAY[0..3] OF REAL;
        END_VAR
        VAR
            i : INT;
            sum : REAL;
        END_VAR
        sum := REAL#0.0;
        FOR i := INT#0 TO INT#3 DO
            sum := sum + Samples[i];
        END_FOR
        Avg4 := sum / REAL#4.0;
        END_FUNCTION

        FUNCTION_BLOCK FB_Valve
        VAR_INPUT
            Enable : BOOL;
            Setpoint : REAL;
        END_VAR
        VAR_OUTPUT
            Open : BOOL;
            Position : REAL;
        END_VAR
        VAR
            ramp : REAL;
        END_VAR
        IF Enable THEN
            Open := TRUE;
            ramp := ClampReal(Setpoint, REAL#0.0, REAL#100.0);
        ELSE
            Open := FALSE;
            ramp := REAL#0.0;
        END_IF
        Position := ramp;
        END_FUNCTION_BLOCK

        PROGRAM Main
        VAR
            Valve : FB_Valve;
            Mode : INT := INT#0;
            CmdEnable : BOOL;
            CmdSetpoint : REAL;
            Samples : ARRAY[0..3] OF REAL;
            Avg : REAL;
            i : INT;
            OpenOut : BOOL;
            PosOut : REAL;
            RefPos : REF_TO REAL;
            Opened : BOOL := FALSE;
            Watchdog : TON;
            WatchdogQ : BOOL;
        END_VAR

        CmdEnable := TRUE;
        CmdSetpoint := REAL#42.5;

        FOR i := INT#0 TO INT#3 DO
            Samples[i] := REAL#15.0;
        END_FOR
        Avg := Avg4(Samples);
        CmdSetpoint := Avg;

        Valve(Enable := CmdEnable, Setpoint := CmdSetpoint, Open => OpenOut, Position => PosOut);
        Opened := OpenOut;

        Watchdog(IN := CmdEnable, PT := T#50ms, Q => WatchdogQ);
        IF WatchdogQ THEN
            Mode := INT#0;
        END_IF

        CASE Mode OF
            INT#0:
                IF Opened THEN
                    Mode := INT#1;
                END_IF
            INT#1:
                Mode := INT#2;
            INT#2:
                Mode := INT#0;
        END_CASE

        RefPos := REF(PosOut);
        RefPos^ := ClampReal(RefPos^, REAL#0.0, REAL#100.0);
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    let result = harness.cycle();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
}

/// Regression test: running the otm_lib motor_variable_speed function block for
/// multiple cycles must not produce a TypeMismatch fault. The user-defined
/// functions UINT_TO_TIME and UDINT_TO_TIME (defined in conversions.st) shadow
/// the stdlib conversion-name pattern, and must be dispatched as user functions.
#[test]
fn otm_lib_motor_variable_speed_no_type_mismatch() {
    let conversions = include_str!("../../../examples/otm_lib/src/conversions.st");
    let udt = include_str!("../../../examples/otm_lib/src/motor_variable_speed_udt.st");
    let pou = include_str!("../../../examples/otm_lib/src/motor_variable_speed_pou.st");
    // Stripped main: remove VAR_EXTERNAL (no CONFIGURATION with VAR_GLOBAL)
    let main_prog = r#"
        PROGRAM Main
        VAR
            motor_pou : motor_variable_speed_pou;
            motor_udt : motor_variable_speed_udt;
        END_VAR
        motor_pou(motor_variable_speed := motor_udt);
        END_PROGRAM
    "#;

    let sources: &[&str] = &[conversions, udt, pou, main_prog];
    let mut harness = TestHarness::from_sources(sources).expect("should compile");
    // Run 10 cycles, advancing 100ms between each to match the real runtime's task interval.
    for i in 1..=10 {
        let result = harness.cycle();
        assert!(
            result.errors.is_empty(),
            "cycle {i} errors: {:?}",
            result.errors
        );
        harness.advance_time(trust_runtime::value::Duration::from_millis(100));
    }
}

/// Regression test: full otm_lib project (with CONFIGURATION and VAR_GLOBAL I/O)
/// must not produce a runtime fault over multiple cycles.
#[test]
fn otm_lib_full_project_no_fault() {
    let conversions = include_str!("../../../examples/otm_lib/src/conversions.st");
    let udt = include_str!("../../../examples/otm_lib/src/motor_variable_speed_udt.st");
    let pou = include_str!("../../../examples/otm_lib/src/motor_variable_speed_pou.st");
    let main_prog = include_str!("../../../examples/otm_lib/src/main.st");
    let config = include_str!("../../../examples/otm_lib/src/config.st");

    let sources: &[&str] = &[conversions, udt, pou, main_prog, config];
    let mut harness = TestHarness::from_sources(sources).expect("should compile");
    for i in 1..=5 {
        let result = harness.cycle();
        assert!(
            result.errors.is_empty(),
            "cycle {i} errors: {:?}",
            result.errors
        );
        harness.advance_time(trust_runtime::value::Duration::from_millis(100));
    }
}
