// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 098da7c84370a4296423abd634e2f8f6f4f84b3edd0a0d3c66a03ff69b2a536b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_150() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test150::Test150Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 150 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test150::Test150State::Pass,
        "Test 150 reached wrong final state"
    );
}
