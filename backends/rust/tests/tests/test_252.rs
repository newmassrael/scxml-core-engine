// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 60da764009afb96185d876c542254f2e8363dba627394829757a2a8f121eddd1
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
use std::time::Duration;

#[test]
fn test_252() {
    let policy = sce_rust_tests::generated::test252::Test252Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 252 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test252::Test252State::Pass,
        "Test 252 reached wrong final state"
    );
}
