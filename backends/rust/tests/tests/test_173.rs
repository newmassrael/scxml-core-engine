// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 32616a8c15423facd5b04f320c1acfc24557f2b58b0b9ca0229cb783903eb112
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test173.scxml:1
use std::time::Duration;

#[test]
fn test_173() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test173::Test173Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 173 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test173::Test173State::Pass,
        "Test 173 reached wrong final state"
    );
}
