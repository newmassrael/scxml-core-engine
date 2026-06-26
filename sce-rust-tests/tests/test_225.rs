// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d43b22670550c67cebe189489d0fdc39f585b0c09803917dea05e0ded254e31e
// generated-at: 1782434360
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test225.scxml:1
use std::time::Duration;

#[test]
fn test_225() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test225::Test225Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 225 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test225::Test225State::Pass,
        "Test 225 reached wrong final state"
    );
}
