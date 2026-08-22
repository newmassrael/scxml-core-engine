// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test531.scxml:1
use std::time::Duration;

#[test]
fn test_531() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test531::Test531Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::http_test_server_url());
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 531 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test531::Test531State::Pass,
        "Test 531 reached wrong final state"
    );
}
