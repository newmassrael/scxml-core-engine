// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 06c80ca1f364d1bd47dcf4355438c9eb8afe054b2712ace900a9053d7a3870aa
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test404.scxml:1
use std::time::Duration;

#[test]
fn test_404() {
    let policy = sce_rust_tests::generated::test404::Test404Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 404 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test404::Test404State::Pass,
        "Test 404 reached wrong final state"
    );
}
