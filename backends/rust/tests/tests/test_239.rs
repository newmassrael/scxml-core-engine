// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 084a969fb5abb3571d5265141500a73eb8505542dc564e6df26ed5160df0909f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test239.scxml:1
use std::time::Duration;

#[test]
fn test_239() {
    let policy = sce_rust_tests::generated::test239::Test239Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 239 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test239::Test239State::Pass,
        "Test 239 reached wrong final state"
    );
}
