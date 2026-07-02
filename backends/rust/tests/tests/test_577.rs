// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: b5e91c83753cb468c86997c5541ac646288562f682111eb4bbd825060d84bc2e
// generated-at: 1782963881
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
use std::time::Duration;

#[test]
fn test_577() {
    let policy = sce_rust_tests::generated::test577::Test577Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 577 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test577::Test577State::Pass,
        "Test 577 reached wrong final state"
    );
}
