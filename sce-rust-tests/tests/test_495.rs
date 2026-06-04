// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: b54483029156719493b67bab1ba0270f7cbbd9e4ba4ab1e2c6d39e74fc9e1571
// generated-at: 1780541051
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test495.scxml:1
use std::time::Duration;

#[test]
fn test_495() {
    let policy = sce_rust_tests::generated::test495::Test495Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 495 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test495::Test495State::Pass,
        "Test 495 reached wrong final state"
    );
}
