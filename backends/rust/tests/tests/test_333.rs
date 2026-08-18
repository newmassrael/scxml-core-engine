// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6316b28d6d1ad6f020128cbcac8380bcbacc703c59c413c6fbd546c800047e63
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test333.scxml:1
use std::time::Duration;

#[test]
fn test_333() {
    let policy = sce_rust_tests::generated::test333::Test333Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 333 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test333::Test333State::Pass,
        "Test 333 reached wrong final state"
    );
}
