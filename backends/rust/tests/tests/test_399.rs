// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 80894143d638a0b7198412ab424baf8aabfb4df3ab8d3543c7c8e64fdb892114
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
