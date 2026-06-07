// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 3acf03cd1e197da0d6a3e7ecc2541747678939372fbe1d99b37c7415a38be32a
// generated-at: 1780830703
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test396.scxml:1
use std::time::Duration;

#[test]
fn test_396() {
    let policy = sce_rust_tests::generated::test396::Test396Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 396 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test396::Test396State::Pass,
        "Test 396 reached wrong final state"
    );
}
