// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 4a741a2915b4fc1d6292d4cc68ddf4af4e269ea63531bfee3c7b94ccd4e9b0bc
// generated-at: 1782562647
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test322.scxml:1
use std::time::Duration;

#[test]
fn test_322() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test322::Test322Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 322 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test322::Test322State::Pass,
        "Test 322 reached wrong final state"
    );
}
