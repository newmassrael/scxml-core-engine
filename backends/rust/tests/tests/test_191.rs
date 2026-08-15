// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2d53d2f6482bd48bbe534a774432c7132f924eed253d3c01ee5b53a731642f97
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test191.scxml:1
use std::time::Duration;

#[test]
fn test_191() {
    let policy = sce_rust_tests::generated::test191::Test191Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 191 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test191::Test191State::Pass,
        "Test 191 reached wrong final state"
    );
}
