// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
use std::time::Duration;

#[test]
fn test_252() {
    let policy = sce_rust_tests::generated::test252::Test252Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    let Ok(()) = engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 252 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test252::Test252State::Pass),
        "Test 252 reached wrong final state"
    );
}
