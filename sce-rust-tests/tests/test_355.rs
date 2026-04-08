// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_355() {
    let policy = sce_rust_tests::generated::test355::Test355Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 355 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test355::Test355State::Pass,
        "Test 355 reached wrong final state"
    );
}
