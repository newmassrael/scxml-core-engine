// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: cf4da7a0913513e15552dabfcd6b53678453b7b4dee1a56eee427fb0db26349a
// generated-at: 1780568753
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test244.scxml:1
use std::time::Duration;

#[test]
fn test_244() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test244::Test244Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 244 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test244::Test244State::Pass,
        "Test 244 reached wrong final state"
    );
}
