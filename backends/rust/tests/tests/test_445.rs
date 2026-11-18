// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test445.scxml:1
use std::time::Duration;

#[test]
fn test_445() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test445::Test445Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 445 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test445::Test445State::Pass),
        "Test 445 reached wrong final state"
    );
}
