// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2337021aa5cf9b8209b5932f23ab0e04a6899271e435f3620bc1da41d7c4d7b7
// generated-at: 1784381543
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test149.scxml:1
use std::time::Duration;

#[test]
fn test_149() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test149::Test149Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 149 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test149::Test149State::Pass,
        "Test 149 reached wrong final state"
    );
}
