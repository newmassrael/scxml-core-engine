// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)
use std::time::Duration;

#[test]
fn test_403a() {
    let policy = sce_rust_tests::generated::test403a::Test403aPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 403a timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test403a::Test403aState::Pass,
        "Test 403a reached wrong final state"
    );
}
