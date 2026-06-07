// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 66bc1c3694f90e60100c842d2a53cd8c05682260c1809ba387d157940d7d6e1d
// generated-at: 1780836426
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test208.scxml:1
use std::time::Duration;

#[test]
fn test_208() {
    let policy = sce_rust_tests::generated::test208::Test208Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 208 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test208::Test208State::Pass,
        "Test 208 reached wrong final state"
    );
}
