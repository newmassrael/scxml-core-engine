// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_314() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test314::Test314Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 314 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test314::Test314State::Pass,
        "Test 314 reached wrong final state"
    );
}
