use super::*;

#[test]
fn encoder_supports_constant_in_expression() {
    let source = r#"
PROGRAM Main
VAR CONSTANT
    Base : INT := 2;
END_VAR
VAR
    result : INT := 0;
END_VAR
result := Base + 3;
END_PROGRAM
"#;

    let module = bytecode_module_from_source(source).unwrap();
    module.validate().unwrap();

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("result", 5);
}

#[test]
fn encoder_supports_constant_in_case_statement() {
    let source = r#"
PROGRAM Main
VAR CONSTANT
    MatchVal : INT := 2;
END_VAR
VAR
    counter : INT := 2;
    result : INT := 0;
END_VAR

CASE counter OF
    MatchVal: result := 1;
ELSE
    result := 0;
END_CASE;
END_PROGRAM
"#;

    let module = bytecode_module_from_source(source).unwrap();
    module.validate().unwrap();

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("result", 1);
}

#[test]
fn encoder_supports_constant_in_array_dimension() {
    let source = r#"
PROGRAM Main
VAR CONSTANT
    Max : INT := 3;
END_VAR
VAR
    arr : ARRAY[0..Max] OF INT;
    result : INT := 0;
END_VAR
arr[Max] := 9;
result := arr[3];
END_PROGRAM
"#;

    let module = bytecode_module_from_source(source).unwrap();
    module.validate().unwrap();

    let strings = match module.section(SectionId::StringTable) {
        Some(SectionData::StringTable(table)) => table,
        other => panic!("expected STRING_TABLE, got {other:?}"),
    };
    let types = match module.section(SectionId::TypeTable) {
        Some(SectionData::TypeTable(table)) => table,
        other => panic!("expected TYPE_TABLE, got {other:?}"),
    };
    let array = find_type(types, strings, "ARRAY[0..3] OF INT");
    match &array.data {
        TypeData::Array { dims, .. } => assert_eq!(dims, &vec![(0, 3)]),
        other => panic!("expected ARRAY type, got {other:?}"),
    }

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("result", 9);
}

#[test]
fn encoder_supports_global_constants_via_var_external_constant() {
    let source = r#"
PROGRAM Main
VAR_EXTERNAL CONSTANT
    G : INT;
END_VAR
VAR
    result : INT := 0;
END_VAR
result := G + 1;
END_PROGRAM

CONFIGURATION Conf
VAR_GLOBAL CONSTANT
    G : INT := 41;
END_VAR
RESOURCE R ON CPU
TASK T (INTERVAL := T#10ms, PRIORITY := 0);
PROGRAM P1 WITH T : Main;
END_RESOURCE
END_CONFIGURATION
"#;

    let module = bytecode_module_from_source(source).unwrap();
    module.validate().unwrap();
    let pou_index = match module.section(SectionId::PouIndex) {
        Some(SectionData::PouIndex(index)) => index,
        other => panic!("expected POU_INDEX, got {other:?}"),
    };
    let bodies = match module.section(SectionId::PouBodies) {
        Some(SectionData::PouBodies(bodies)) => bodies,
        other => panic!("expected POU_BODIES, got {other:?}"),
    };
    let program = pou_index
        .entries
        .iter()
        .find(|entry| entry.kind == PouKind::Program && entry.code_length > 0)
        .expect("program entry");
    let code =
        &bodies[program.code_offset as usize..(program.code_offset + program.code_length) as usize];
    let opcodes = collect_opcodes(code);
    assert!(!opcodes.is_empty());
    assert!(opcodes.contains(&0x20)); // load ref
    assert!(opcodes.contains(&0x21)); // store ref
    assert!(opcodes.contains(&0x40)); // add
}

#[test]
fn encoder_supports_method_constant_overwrite() {
    let source = r#"
FUNCTION_BLOCK FB
VAR CONSTANT
    C : INT := 10;
END_VAR
METHOD PUBLIC Value : INT
VAR CONSTANT
    C : INT := 2;
END_VAR
Value := C + 3;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM Main
VAR
    fb : FB;
    result : INT := 0;
END_VAR
result := fb.Value();
END_PROGRAM
"#;

    let module = bytecode_module_from_source(source).unwrap();
    module.validate().unwrap();

    let mut harness = TestHarness::from_source(source).unwrap();
    harness.cycle();
    harness.assert_eq("result", 5);
}
