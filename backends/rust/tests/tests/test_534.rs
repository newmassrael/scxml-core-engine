// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2eaf0bfe80897ae515b0d732a8bb3914baa7c870ee8dd206a0a3dbc4956501d1
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test534.scxml:1
use std::time::Duration;

#[test]
fn test_534() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test534::Test534Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::HTTP_TEST_SERVER_URL);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 534 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test534::Test534State::Pass,
        "Test 534 reached wrong final state"
    );
}
