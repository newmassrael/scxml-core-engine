// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: b54483029156719493b67bab1ba0270f7cbbd9e4ba4ab1e2c6d39e74fc9e1571
// generated-at: 1780541051
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test509.scxml:1
use std::time::Duration;

#[test]
fn test_509() {
    let policy = sce_rust_tests::generated::test509::Test509Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 509 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test509::Test509State::Pass,
        "Test 509 reached wrong final state"
    );
}
