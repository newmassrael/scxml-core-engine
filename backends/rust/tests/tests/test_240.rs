// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525842
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test240.scxml:1
use std::time::Duration;

#[test]
fn test_240() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test240::Test240Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 240 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test240::Test240State::Pass,
        "Test 240 reached wrong final state"
    );
}
