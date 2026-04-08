// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_558() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test558::Test558Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 558 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test558::Test558State::Pass,
        "Test 558 reached wrong final state"
    );
}
