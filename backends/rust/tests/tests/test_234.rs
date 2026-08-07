// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0979cc035025f602036307b1c3d608d3deba515781f8b4aa24c2aff0a8a41fe0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test234.scxml:1
use std::time::Duration;

#[test]
fn test_234() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test234::Test234Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 234 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test234::Test234State::Pass,
        "Test 234 reached wrong final state"
    );
}
