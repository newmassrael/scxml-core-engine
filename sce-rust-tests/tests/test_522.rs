// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5af0768adc0cd444b401fc40536c0de87cadf9b1f8be7299536f4fc9ed22e337
// generated-at: 1780020097
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test522.scxml:1
use std::time::Duration;

#[test]
fn test_522() {
    let policy = sce_rust_tests::generated::test522::Test522Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 522 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test522::Test522State::Pass,
        "Test 522 reached wrong final state"
    );
}
