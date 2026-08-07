// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c56c70704f5f2bb35214b9afedb17072586ef770e8f707a00c3cc931286b4b25
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test399.scxml:1
use std::time::Duration;

#[test]
fn test_399() {
    let policy = sce_rust_tests::generated::test399::Test399Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 399 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test399::Test399State::Pass,
        "Test 399 reached wrong final state"
    );
}
