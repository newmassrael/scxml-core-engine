// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 0ae95bdc8568e54ab8b0becbe6b9dbf13fd2de6976e2b75ba52db7079781e01f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_411() {
    let policy = sce_rust_tests::generated::test411::Test411Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 411 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test411::Test411State::Pass,
        "Test 411 reached wrong final state"
    );
}
