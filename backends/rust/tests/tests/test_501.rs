// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c44eb8ea1f7a6700f381c20ea1b37f015805c8beff30d4d12e22d7c96e5e1124
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test501.scxml:1
use std::time::Duration;

#[test]
fn test_501() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test501::Test501Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 501 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test501::Test501State::Pass,
        "Test 501 reached wrong final state"
    );
}
