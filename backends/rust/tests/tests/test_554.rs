// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test554.scxml:1
use std::time::Duration;

#[test]
fn test_554() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test554::Test554Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 554 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test554::Test554State::Pass),
        "Test 554 reached wrong final state"
    );
}
