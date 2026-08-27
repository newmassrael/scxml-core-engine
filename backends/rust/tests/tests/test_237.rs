// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0afb1f0f0230f40c373aa80a890f61f2cc90b35724e7d86493a9e44e197b2d1b
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
