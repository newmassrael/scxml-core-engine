// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1f4fc251a4bb4df71320b116cc055aa1687156c3a3402c346abf1bd3694d0437
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test522.scxml:1
use std::time::Duration;

#[test]
fn test_522() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test522::Test522Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::http_test_server_url());
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 522 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test522::Test522State::Pass,
        "Test 522 reached wrong final state"
    );
}
