// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b90187ddc6ef966a857dd727ee00a2afc70a676ffdaa3e71c82f25c4e9c20678
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test237.scxml:1
use std::time::Duration;

#[test]
fn test_237() {
    let policy = sce_rust_tests::generated::test237::Test237Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 237 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test237::Test237State::Pass,
        "Test 237 reached wrong final state"
    );
}
