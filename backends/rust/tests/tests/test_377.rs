// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test377.scxml:1
use std::time::Duration;

#[test]
fn test_377() {
    let policy = sce_rust_tests::generated::test377::Test377Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    let Ok(()) = engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 377 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test377::Test377State::Pass),
        "Test 377 reached wrong final state"
    );
}
