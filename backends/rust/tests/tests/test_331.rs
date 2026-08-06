// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2eaf0bfe80897ae515b0d732a8bb3914baa7c870ee8dd206a0a3dbc4956501d1
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test331.scxml:1
use std::time::Duration;

#[test]
fn test_331() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test331::Test331Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 331 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test331::Test331State::Pass,
        "Test 331 reached wrong final state"
    );
}
