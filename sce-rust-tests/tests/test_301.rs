// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7197aba2d6c1e9ac23142e9725b0b92b81ba30d30925873778f22c9cb1e581d7
// generated-at: 1780548564
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test301.scxml:1
use std::time::Duration;

#[test]
fn test_301() {
    let policy = sce_rust_tests::generated::test301::Test301Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 301 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test301::Test301State::Pass,
        "Test 301 reached wrong final state"
    );
}
