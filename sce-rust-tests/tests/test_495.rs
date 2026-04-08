// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_495() {
    let policy = sce_rust_tests::generated::test495::Test495Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 495 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test495::Test495State::Pass,
        "Test 495 reached wrong final state"
    );
}
