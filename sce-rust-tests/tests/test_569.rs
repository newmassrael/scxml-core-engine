// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031381
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test569.scxml:1
use std::time::Duration;

#[test]
fn test_569() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test569::Test569Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 569 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test569::Test569State::Pass,
        "Test 569 reached wrong final state"
    );
}
