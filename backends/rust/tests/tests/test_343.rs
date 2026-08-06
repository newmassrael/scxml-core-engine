// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 97155411577c11a793c509f8eca92ed763090b054ea00a1be8ebdfe84ee878d0
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test343.scxml:1
use std::time::Duration;

#[test]
fn test_343() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test343::Test343Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 343 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test343::Test343State::Pass,
        "Test 343 reached wrong final state"
    );
}
