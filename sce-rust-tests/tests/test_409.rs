// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564442
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
use std::time::Duration;

#[test]
fn test_409() {
    let policy = sce_rust_tests::generated::test409::Test409Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 409 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test409::Test409State::Pass,
        "Test 409 reached wrong final state"
    );
}
