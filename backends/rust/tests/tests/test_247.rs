// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test247.scxml:1
use std::time::Duration;

#[test]
fn test_247() {
    let policy = sce_rust_tests::generated::test247::Test247Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 247 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test247::Test247State::Pass,
        "Test 247 reached wrong final state"
    );
}
