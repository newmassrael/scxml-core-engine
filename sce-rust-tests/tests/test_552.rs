// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 09c66e0a06202a6ec53b4591ac58670a6615a699910ff161304360792e1e7915
// generated-at: 1780732000
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test552.scxml:1
use std::time::Duration;

#[test]
fn test_552() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test552::Test552Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 552 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test552::Test552State::Pass,
        "Test 552 reached wrong final state"
    );
}
