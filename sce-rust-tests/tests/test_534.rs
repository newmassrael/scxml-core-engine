// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 001c18dbd894d9aea04585450d4d20eb017bbd93c8a0ad242a38dda5c53478ac
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
use std::time::Duration;

#[test]
fn test_534() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test534::Test534Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 534 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test534::Test534State::Pass,
        "Test 534 reached wrong final state"
    );
}
