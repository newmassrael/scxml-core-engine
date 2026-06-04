// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test403a.scxml:1
use std::time::Duration;

#[test]
fn test_403a() {
    let policy = sce_rust_tests::generated::test403a::Test403aPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 403a timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test403a::Test403aState::Pass,
        "Test 403a reached wrong final state"
    );
}
