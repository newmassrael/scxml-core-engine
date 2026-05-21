// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: ce261274019ce48077782e7ee06e70f44649cd64bd8924b568aaf0ee8f281e9d
// generated-at: 1779371070
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test444.scxml:1
use std::time::Duration;

#[test]
fn test_444() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test444::Test444Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 444 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test444::Test444State::Pass,
        "Test 444 reached wrong final state"
    );
}
