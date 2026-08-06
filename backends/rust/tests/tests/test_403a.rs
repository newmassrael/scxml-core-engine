// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 670395eefe7272d78e62bf7a7fd9181e96e4a744175a58a4c4de1240c73f57bc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test403a.scxml:1
use std::time::Duration;

#[test]
fn test_403a() {
    let policy = sce_rust_tests::generated::test403a::Test403aPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 403a timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test403a::Test403aState::Pass,
        "Test 403a reached wrong final state"
    );
}
