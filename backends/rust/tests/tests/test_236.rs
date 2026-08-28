// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2999a09c910b968e408271dc62f423daf659e11e3dbdea0cdf9857029573f331
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test236.scxml:1
use std::time::Duration;

#[test]
fn test_236() {
    let policy = sce_rust_tests::generated::test236::Test236Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 236 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test236::Test236State::Pass,
        "Test 236 reached wrong final state"
    );
}
