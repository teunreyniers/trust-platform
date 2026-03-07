#[cfg(feature = "legacy-interpreter")]
use trust_runtime::execution_backend::ExecutionBackend;
use trust_runtime::harness::TestHarness;

#[test]
fn assign_attempt() {
    let source = r#"
        PROGRAM Test
        VAR
            x : INT := INT#5;
            r1 : REF_TO INT;
            r2 : REF_TO INT;
            out : INT := INT#0;
        END_VAR
        r1 := REF(x);
        r2 ?= r1;
        out := r2^;
        r2 ?= NULL;
        IF r2 = NULL THEN
            out := out + INT#1;
        END_IF;
        END_PROGRAM
    "#;

    let mut harness = TestHarness::from_source(source).unwrap();
    #[cfg(feature = "legacy-interpreter")]
    harness
        .runtime_mut()
        .set_execution_backend(ExecutionBackend::Interpreter)
        .expect("switch to interpreter backend");
    harness.cycle();
    harness.assert_eq("out", 6i16);
}
