// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_456() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test456::Test456Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 456 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test456::Test456State::Pass,
        "Test 456 reached wrong final state"
    );
}
