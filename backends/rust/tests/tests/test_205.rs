// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367094
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test205.scxml:1
use std::time::Duration;

#[test]
fn test_205() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test205::Test205Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 205 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test205::Test205State::Pass,
        "Test 205 reached wrong final state"
    );
}
