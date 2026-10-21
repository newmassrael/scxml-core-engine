// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test510.scxml:1
use std::time::Duration;

#[test]
fn test_510() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test510::Test510Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::http_test_server_url());
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 510 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test510::Test510State::Pass),
        "Test 510 reached wrong final state"
    );
}
