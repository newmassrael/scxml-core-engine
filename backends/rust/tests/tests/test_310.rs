// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0979cc035025f602036307b1c3d608d3deba515781f8b4aa24c2aff0a8a41fe0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test310.scxml:1
use std::time::Duration;

#[test]
fn test_310() {
    let policy = sce_rust_tests::generated::test310::Test310Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 310 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test310::Test310State::Pass,
        "Test 310 reached wrong final state"
    );
}
