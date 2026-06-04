// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7197aba2d6c1e9ac23142e9725b0b92b81ba30d30925873778f22c9cb1e581d7
// generated-at: 1780548564
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test531.scxml:1
use std::time::Duration;

#[test]
fn test_531() {
    let policy = sce_rust_tests::generated::test531::Test531Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 531 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test531::Test531State::Pass,
        "Test 531 reached wrong final state"
    );
}
