// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f5e6315f2ec211d36d839290b90cbd833e902936cc9328b605b51a480ada76bd
// generated-at: 1779411567
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test449.scxml:1
use std::time::Duration;

#[test]
fn test_449() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test449::Test449Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 449 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test449::Test449State::Pass,
        "Test 449 reached wrong final state"
    );
}
