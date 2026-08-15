// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e136547eba5b1b26d444df3b244f86733d75a97e370ef305f7a135f66e51e2c8
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test183.scxml:1
use std::time::Duration;

#[test]
fn test_183() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test183::Test183Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 183 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test183::Test183State::Pass,
        "Test 183 reached wrong final state"
    );
}
