// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d43b22670550c67cebe189489d0fdc39f585b0c09803917dea05e0ded254e31e
// generated-at: 1782434360
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test200.scxml:1
use std::time::Duration;

#[test]
fn test_200() {
    let policy = sce_rust_tests::generated::test200::Test200Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 200 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test200::Test200State::Pass,
        "Test 200 reached wrong final state"
    );
}
