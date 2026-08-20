// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 63129ea5a60cce4407210a3c2e3ff224327767ebf6618c3f4ed41b0a49b7454d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test510.scxml:1
use std::time::Duration;

#[test]
fn test_510() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = sce_rust_tests::generated::test510::Test510Policy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::HTTP_TEST_SERVER_URL);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 510 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test510::Test510State::Pass,
        "Test 510 reached wrong final state"
    );
}
