// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)
use std::time::Duration;

#[test]
fn test_404() {
    let policy = sce_rust_tests::generated::test404::Test404Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 404 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test404::Test404State::Pass,
        "Test 404 reached wrong final state"
    );
}
