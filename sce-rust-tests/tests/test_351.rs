// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bee566d0969cba6048cf66f73f5f775d02dafd3fb011e32cfb151e43f5c41677
// generated-at: 1779444435
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test351.scxml:1
use std::time::Duration;

#[test]
fn test_351() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test351::Test351Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 351 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test351::Test351State::Pass,
        "Test 351 reached wrong final state"
    );
}
