// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980217
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test329.scxml:1
use std::time::Duration;

#[test]
fn test_329() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test329::Test329Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 329 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test329::Test329State::Pass,
        "Test 329 reached wrong final state"
    );
}
