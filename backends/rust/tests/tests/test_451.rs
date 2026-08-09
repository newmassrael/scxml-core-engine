// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f5fde488bb26d050ed6ca4285c6964cc031a9d1311486db8d9c07efbb803316f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test451.scxml:1
use std::time::Duration;

#[test]
fn test_451() {
    let policy = sce_rust_tests::generated::test451::Test451Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 451 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test451::Test451State::Pass,
        "Test 451 reached wrong final state"
    );
}
