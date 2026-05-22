// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e798da33d5279236b681cdea18a53a3971a9b769ae5a0bc652a7f8fc89ca7b27
// generated-at: 1779450894
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test198.scxml:1
use std::time::Duration;

#[test]
fn test_198() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test198::Test198Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 198 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test198::Test198State::Pass,
        "Test 198 reached wrong final state"
    );
}
