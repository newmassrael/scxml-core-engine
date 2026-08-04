// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4c41e247b67cf81983b818aeaf91bedaefbcf466fafdc8dd55875b0211b1bb15
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test411.scxml:1
use std::time::Duration;

#[test]
fn test_411() {
    let policy = sce_rust_tests::generated::test411::Test411Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 411 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test411::Test411State::Pass,
        "Test 411 reached wrong final state"
    );
}
