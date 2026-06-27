// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: dade9f8de6d0c296ea9dd537c4a48e14404d516e6b96273faf48e4d26f58db4f
// generated-at: 1782564442
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test155.scxml:1
use std::time::Duration;

#[test]
fn test_155() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test155::Test155Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 155 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test155::Test155State::Pass,
        "Test 155 reached wrong final state"
    );
}
