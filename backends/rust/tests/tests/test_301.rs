// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test301.scxml:1
use std::time::Duration;

#[test]
fn test_301() {
    let policy = sce_rust_tests::generated::test301::Test301Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 301 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test301::Test301State::Pass),
        "Test 301 reached wrong final state"
    );
}
