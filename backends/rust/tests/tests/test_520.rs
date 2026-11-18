// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e40b95c2d842dcf564705fa3efcf7ff2175df2e7887977f32be9e362c2a3f25
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test520.scxml:1
use std::time::Duration;

#[test]
fn test_520() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test520::Test520Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::http_test_server_url());
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 520 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test520::Test520State::Pass),
        "Test 520 reached wrong final state"
    );
}
