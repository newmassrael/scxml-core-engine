// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_520() {
    let policy = sce_rust_tests::generated::test520::Test520Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 520 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test520::Test520State::Pass,
        "Test 520 reached wrong final state"
    );
}
