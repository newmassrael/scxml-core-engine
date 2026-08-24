// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test518.scxml:1
use std::time::Duration;

#[test]
fn test_518() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test518::Test518Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::http_test_server_url());
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 518 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test518::Test518State::Pass,
        "Test 518 reached wrong final state"
    );
}
