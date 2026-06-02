// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e8782a5c8351481fc8f6e7fcdb09caae80cbe9e47c6019dcf15afff703e3c3b3
// generated-at: 1780407549
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test313.scxml:1
use std::time::Duration;

#[test]
fn test_313() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test313::Test313Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 313 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test313::Test313State::Pass,
        "Test 313 reached wrong final state"
    );
}
