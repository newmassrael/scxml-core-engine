// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4cbf0ce468f2db0011b4fa010e6c117357964548e492f95e76a21755c70778e3
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test335.scxml:1
use std::time::Duration;

#[test]
fn test_335() {
    let policy = sce_rust_tests::generated::test335::Test335Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 335 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test335::Test335State::Pass,
        "Test 335 reached wrong final state"
    );
}
