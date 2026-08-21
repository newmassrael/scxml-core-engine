// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f7291ab6d7896ee95dd448a8f7fc2759f6a0259c69bcc8f54f868651f4b8fe72
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test567.scxml:1
use std::time::Duration;

#[test]
fn test_567() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test567::Test567Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::HTTP_TEST_SERVER_URL);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 567 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test567::Test567State::Pass,
        "Test 567 reached wrong final state"
    );
}
