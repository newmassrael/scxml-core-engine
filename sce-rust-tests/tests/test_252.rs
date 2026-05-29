// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5e63a3ecc19b397697c3e24d727bc3c78cb748941f07d7f7c9d76cdea58d15a4
// generated-at: 1780032747
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
use std::time::Duration;

#[test]
fn test_252() {
    let policy = sce_rust_tests::generated::test252::Test252Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 252 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test252::Test252State::Pass,
        "Test 252 reached wrong final state"
    );
}
