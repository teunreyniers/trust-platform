use trust_runtime::harness::TestHarness;

#[test]
fn advance_time_and_set_time_update_runtime_clock_in_test_function_block() {
    let source = r#"
PROGRAM Main
END_PROGRAM

TEST_FUNCTION_BLOCK ClockControl
VAR
    after_advance : TIME;
    after_set : TIME;
END_VAR
ADVANCE_TIME(T#25ms);
after_advance := TIME();
SET_TIME(T#100ms);
after_set := TIME();
ASSERT_EQUAL(T#25ms, after_advance);
ASSERT_EQUAL(T#100ms, after_set);
END_TEST_FUNCTION_BLOCK
"#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness
        .runtime_mut()
        .execute_function_block_by_name("ClockControl")
        .unwrap();
}

#[test]
fn advance_time_enables_ton_completion_in_test_function_block() {
    let source = r#"
PROGRAM Main
END_PROGRAM

TEST_FUNCTION_BLOCK TonWithAdvance
VAR
    t : TON;
END_VAR
t(IN := TRUE, PT := T#5ms);
ADVANCE_TIME(T#5ms);
t(IN := TRUE, PT := T#5ms);
ASSERT_TRUE(t.Q);
END_TEST_FUNCTION_BLOCK
"#;

    let mut harness = TestHarness::from_source(source).unwrap();
    harness
        .runtime_mut()
        .execute_function_block_by_name("TonWithAdvance")
        .unwrap();
}
