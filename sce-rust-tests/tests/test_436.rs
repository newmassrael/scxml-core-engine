// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568711
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test436.scxml:1
use std::time::Duration;

#[test]
fn test_436() {
    let policy = sce_rust_tests::generated::test436::Test436Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 436 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test436::Test436State::Pass,
        "Test 436 reached wrong final state"
    );
}
