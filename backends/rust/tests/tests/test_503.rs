// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b1f5842221aea79fe7d00a79a5e0a1c9bb465b536d392d5c439d8f7ec5538edd
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test503.scxml:1
use std::time::Duration;

#[test]
fn test_503() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test503::Test503Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 503 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test503::Test503State::Pass,
        "Test 503 reached wrong final state"
    );
}
