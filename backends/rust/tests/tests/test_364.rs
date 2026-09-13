// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d4e22b42c8fb86f84b0969d1316ae4d81aa27fbb4a4a2d70f49968ff55378fdc
// generated-at: 0
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
