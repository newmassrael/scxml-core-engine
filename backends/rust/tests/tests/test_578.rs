// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 27e838bbd8708f09c9261661bfb19da674340e525b736fa0c3611ebf1187751e
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test578.scxml:1
use std::time::Duration;

#[test]
fn test_578() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test578::Test578Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 578 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test578::Test578State::Pass,
        "Test 578 reached wrong final state"
    );
}
