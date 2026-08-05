// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 9fc57a6c741c08949d8257dc6586d295bb33ea67c2f5a0208405c69dc89fe99d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test149.scxml:1
use std::time::Duration;

#[test]
fn test_149() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test149::Test149Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 149 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test149::Test149State::Pass,
        "Test 149 reached wrong final state"
    );
}
