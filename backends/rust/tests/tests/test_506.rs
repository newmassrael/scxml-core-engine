// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test506.scxml:1
use std::time::Duration;

#[test]
fn test_506() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test506::Test506Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 506 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test506::Test506State::Pass),
        "Test 506 reached wrong final state"
    );
}
