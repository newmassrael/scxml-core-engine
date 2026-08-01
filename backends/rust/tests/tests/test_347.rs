// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: daa56c2f4afb81deb723d1d6725c872edb8b62d3d9c4a93c07c834af3417504f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test347.scxml:1
use std::time::Duration;

#[test]
fn test_347() {
    let policy = sce_rust_tests::generated::test347::Test347Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 347 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test347::Test347State::Pass,
        "Test 347 reached wrong final state"
    );
}
