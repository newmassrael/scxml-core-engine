// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_333() {
    let policy = sce_rust_tests::generated::test333::Test333Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 333 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test333::Test333State::Pass,
        "Test 333 reached wrong final state"
    );
}
