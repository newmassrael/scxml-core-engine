// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_403c() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test403c::Test403cPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 403c timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test403c::Test403cState::Pass,
        "Test 403c reached wrong final state"
    );
}
