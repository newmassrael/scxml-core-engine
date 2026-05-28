// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980217
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test201.scxml:1
use std::time::Duration;

#[test]
fn test_201() {
    let policy = sce_rust_tests::generated::test201::Test201Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 201 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test201::Test201State::Pass,
        "Test 201 reached wrong final state"
    );
}
