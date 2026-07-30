// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 82d5a5b31a2776e65c97ff666726e5d471238b15131eddc7520023d807e91b34
// generated-at: 1785371280
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test421.scxml:1
use std::time::Duration;

#[test]
fn test_421() {
    let policy = sce_rust_tests::generated::test421::Test421Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 421 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test421::Test421State::Pass,
        "Test 421 reached wrong final state"
    );
}
