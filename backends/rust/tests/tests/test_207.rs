// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test207.scxml:1
use std::time::Duration;

#[test]
fn test_207() {
    let policy = sce_rust_tests::generated::test207::Test207Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 207 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test207::Test207State::Pass),
        "Test 207 reached wrong final state"
    );
}
