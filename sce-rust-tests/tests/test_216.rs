// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081954
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test216.scxml:1
use std::time::Duration;

#[test]
fn test_216() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test216::Test216Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 216 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test216::Test216State::Pass,
        "Test 216 reached wrong final state"
    );
}
