// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985212
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test500.scxml:1
use std::time::Duration;

#[test]
fn test_500() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test500::Test500Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 500 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test500::Test500State::Pass,
        "Test 500 reached wrong final state"
    );
}
