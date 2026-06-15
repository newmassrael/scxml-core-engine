// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 030a39123c8149accb30146fc4a4999b6e8826a330653d219a562116c552e0d8
// generated-at: 1781483327
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test339.scxml:1
use std::time::Duration;

#[test]
fn test_339() {
    let policy = sce_rust_tests::generated::test339::Test339Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 339 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test339::Test339State::Pass,
        "Test 339 reached wrong final state"
    );
}
