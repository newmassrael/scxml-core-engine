// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_578() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test578::Test578Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 578 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test578::Test578State::Pass,
        "Test 578 reached wrong final state"
    );
}
