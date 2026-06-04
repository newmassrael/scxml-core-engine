// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e03d007af0e666370768a5b0be76775e8be2eb913728a32c0bf7ae79d6929af0
// generated-at: 1780566007
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test189.scxml:1
use std::time::Duration;

#[test]
fn test_189() {
    let policy = sce_rust_tests::generated::test189::Test189Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 189 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test189::Test189State::Pass,
        "Test 189 reached wrong final state"
    );
}
