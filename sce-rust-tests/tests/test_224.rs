// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379900
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test224.scxml:1
use std::time::Duration;

#[test]
fn test_224() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test224::Test224Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 224 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test224::Test224State::Pass,
        "Test 224 reached wrong final state"
    );
}
