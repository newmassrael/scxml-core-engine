// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_513() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test513::Test513Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 513 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test513::Test513State::Pass,
        "Test 513 reached wrong final state"
    );
}
