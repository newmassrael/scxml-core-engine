// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test294.scxml:1
use std::time::Duration;

#[test]
fn test_294() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test294::Test294Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 294 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test294::Test294State::Pass,
        "Test 294 reached wrong final state"
    );
}
