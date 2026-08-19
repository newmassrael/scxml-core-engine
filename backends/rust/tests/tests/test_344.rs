// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e1ef1a80ec6f1d98421ed2b76701aed66a2f64164d943082fb9a22d750e546a9
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test344.scxml:1
use std::time::Duration;

#[test]
fn test_344() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test344::Test344Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 344 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test344::Test344State::Pass,
        "Test 344 reached wrong final state"
    );
}
