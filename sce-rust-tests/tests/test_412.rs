// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)
use std::time::Duration;

#[test]
fn test_412() {
    let policy = sce_rust_tests::generated::test412::Test412Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 412 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test412::Test412State::Pass,
        "Test 412 reached wrong final state"
    );
}
