// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c496f893fb4def171deba817f047a2a335356d181c631fa74825a157a7412c3e
// generated-at: 1784370262
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test533.scxml:1
use std::time::Duration;

#[test]
fn test_533() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test533::Test533Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 533 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test533::Test533State::Pass,
        "Test 533 reached wrong final state"
    );
}
