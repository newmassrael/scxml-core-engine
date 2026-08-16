// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6b3d1716c5fe7bf441783d277357c458e7e14d8fc3f1d3e67e7f0181f437b229
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test496.scxml:1
use std::time::Duration;

#[test]
fn test_496() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test496::Test496Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 496 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test496::Test496State::Pass,
        "Test 496 reached wrong final state"
    );
}
