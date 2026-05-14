// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 38f9aa1b2d3ebbd296494a87466e863947e9800e212f92f4427f69cce23376aa
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_388() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test388::Test388Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 388 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test388::Test388State::Pass,
        "Test 388 reached wrong final state"
    );
}
