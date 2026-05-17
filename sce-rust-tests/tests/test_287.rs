// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778994568
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test287.scxml:1
use std::time::Duration;

#[test]
fn test_287() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test287::Test287Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 287 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test287::Test287State::Pass,
        "Test 287 reached wrong final state"
    );
}
