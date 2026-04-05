// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)
use std::time::Duration;

#[test]
fn test_156() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test156::Test156Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 156 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test156::Test156State::Pass,
        "Test 156 reached wrong final state"
    );
}
