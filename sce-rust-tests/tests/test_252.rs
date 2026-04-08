// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_252() {
    let policy = sce_rust_tests::generated::test252::Test252Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 252 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test252::Test252State::Pass,
        "Test 252 reached wrong final state"
    );
}
