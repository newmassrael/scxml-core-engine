// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 07a1057b89512b0ade7260ce662ea4e6ef3c2abde2d5bd32fb4fe82bd263d4bc
// generated-at: 1780802714
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test532.scxml:1
use std::time::Duration;

#[test]
fn test_532() {
    let policy = sce_rust_tests::generated::test532::Test532Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 532 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test532::Test532State::Pass,
        "Test 532 reached wrong final state"
    );
}
