// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ae61d1957de25e0b25f834d19f5248615526e219f80f117e4ba216dd462396d0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test436.scxml:1
use std::time::Duration;

#[test]
fn test_436() {
    let policy = sce_rust_tests::generated::test436::Test436Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 436 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test436::Test436State::Pass,
        "Test 436 reached wrong final state"
    );
}
