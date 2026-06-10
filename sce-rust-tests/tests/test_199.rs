// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f835a323a3abc9cebc80341e1840b22b95739a2efa1726ad2c440477eff36482
// generated-at: 1781089257
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test199.scxml:1
use std::time::Duration;

#[test]
fn test_199() {
    let policy = sce_rust_tests::generated::test199::Test199Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 199 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test199::Test199State::Pass,
        "Test 199 reached wrong final state"
    );
}
