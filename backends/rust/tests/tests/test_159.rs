// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 817d9c061804919d9748138703a11f334a156a4e2a1e5a3c66f1c4e7ca554aa2
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test159.scxml:1
use std::time::Duration;

#[test]
fn test_159() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test159::Test159Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 159 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test159::Test159State::Pass,
        "Test 159 reached wrong final state"
    );
}
