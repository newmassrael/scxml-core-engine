// GENERATED -- DO NOT EDIT (generate_rust_w3c.py)
use std::time::Duration;

#[test]
fn test_240() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test240::Test240Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 240 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test240::Test240State::Pass,
        "Test 240 reached wrong final state"
    );
}
