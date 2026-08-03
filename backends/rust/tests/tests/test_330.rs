// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a721af75373ae9de49c4cdea1acca1394bb60a4994ec71ccf7cd0c509dda74e7
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test330.scxml:1
use std::time::Duration;

#[test]
fn test_330() {
    let policy = sce_rust_tests::generated::test330::Test330Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 330 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test330::Test330State::Pass,
        "Test 330 reached wrong final state"
    );
}
