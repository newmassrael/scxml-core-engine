// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606837
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test333.scxml:1
use std::time::Duration;

#[test]
fn test_333() {
    let policy = sce_rust_tests::generated::test333::Test333Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 333 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test333::Test333State::Pass,
        "Test 333 reached wrong final state"
    );
}
