// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test449.scxml:1
use std::time::Duration;

#[test]
fn test_449() {
    let policy = sce_rust_tests::generated::test449::Test449Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 449 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test449::Test449State::Pass),
        "Test 449 reached wrong final state"
    );
}
