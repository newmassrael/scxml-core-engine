// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 73644a8c52ee83b6af224889edefc07c66120d6db7d21a41c918be4815ed8509
// generated-at: 1779022531
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test527.scxml:1
use std::time::Duration;

#[test]
fn test_527() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test527::Test527Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 527 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test527::Test527State::Pass,
        "Test 527 reached wrong final state"
    );
}
