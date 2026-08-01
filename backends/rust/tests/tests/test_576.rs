// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
use std::time::Duration;

#[test]
fn test_576() {
    let policy = sce_rust_tests::generated::test576::Test576Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 576 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test576::Test576State::Pass,
        "Test 576 reached wrong final state"
    );
}
