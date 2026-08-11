// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 56bec87d0124f368b72ecb45f170dc38a324027a2fa3663195c8aeaa13f5d24d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test413.scxml:1
use std::time::Duration;

#[test]
fn test_413() {
    let policy = sce_rust_tests::generated::test413::Test413Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 413 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test413::Test413State::Pass,
        "Test 413 reached wrong final state"
    );
}
