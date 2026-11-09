// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test243.scxml:1
use std::time::Duration;

#[test]
fn test_243() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test243::Test243Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 243 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test243::Test243State::Pass),
        "Test 243 reached wrong final state"
    );
}
