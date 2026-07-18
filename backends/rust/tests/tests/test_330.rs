// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c5e718a965673d48d2d901bab6814a883b52bbad31500159c63233aec229e0ef
// generated-at: 1784388944
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test330.scxml:1
use std::time::Duration;

#[test]
fn test_330() {
    let policy = sce_rust_tests::generated::test330::Test330Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 330 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test330::Test330State::Pass,
        "Test 330 reached wrong final state"
    );
}
