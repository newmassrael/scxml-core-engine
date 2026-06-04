// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 6f9dfe10efef0bb8941aa4cdcfc3ee5783e2349124ce8972e5dc402e99e79f39
// generated-at: 1780582368
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test448.scxml:1
use std::time::Duration;

#[test]
fn test_448() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test448::Test448Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 448 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test448::Test448State::Pass,
        "Test 448 reached wrong final state"
    );
}
