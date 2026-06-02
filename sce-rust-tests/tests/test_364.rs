// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test364.scxml:1
use std::time::Duration;

#[test]
fn test_364() {
    let policy = sce_rust_tests::generated::test364::Test364Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 364 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test364::Test364State::Pass,
        "Test 364 reached wrong final state"
    );
}
