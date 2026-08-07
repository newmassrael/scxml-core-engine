// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 3f00b6ad29c2eff5bb5558a6167abdac4572045d11f8d695901879b002032c6b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test452.scxml:1
use std::time::Duration;

#[test]
fn test_452() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test452::Test452Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 452 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test452::Test452State::Pass,
        "Test 452 reached wrong final state"
    );
}
