// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d6df7c5cb569a8142d0ee296b73fd46e2cbd91d66a31cab131337d70b3fd380b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
use std::time::Duration;

#[test]
fn test_577() {
    let policy = sce_rust_tests::generated::test577::Test577Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 577 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test577::Test577State::Pass,
        "Test 577 reached wrong final state"
    );
}
