// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 30c21c2126baf025b95abcba3754b0bcf0280066f6d16a0568643a49c1942e1f
// generated-at: 1779967138
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test387.scxml:1
use std::time::Duration;

#[test]
fn test_387() {
    let policy = sce_rust_tests::generated::test387::Test387Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 387 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test387::Test387State::Pass,
        "Test 387 reached wrong final state"
    );
}
