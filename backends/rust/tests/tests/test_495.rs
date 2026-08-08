// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 82c2acf711bf144ff090617437c4b567e5f475de105d2b1d2386e3cb7f2a1451
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test495.scxml:1
use std::time::Duration;

#[test]
fn test_495() {
    let policy = sce_rust_tests::generated::test495::Test495Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 495 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test495::Test495State::Pass,
        "Test 495 reached wrong final state"
    );
}
