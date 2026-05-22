// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test415.scxml:1
use std::time::Duration;

#[test]
fn test_415() {
    let policy = sce_rust_tests::generated::test415::Test415Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 415 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test415::Test415State::Final,
        "Test 415 reached wrong final state"
    );
}
