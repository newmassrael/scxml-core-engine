// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779589481
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
