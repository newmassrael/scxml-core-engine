// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 001c18dbd894d9aea04585450d4d20eb017bbd93c8a0ad242a38dda5c53478ac
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_399() {
    let policy = sce_rust_tests::generated::test399::Test399Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 399 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test399::Test399State::Pass,
        "Test 399 reached wrong final state"
    );
}
