// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 94a6daf42142517c0ee9ba49a95e1db9d84d30a097beabea83a903e1d7ba88bf
// generated-at: 1779020074
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test378.scxml:1
use std::time::Duration;

#[test]
fn test_378() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test378::Test378Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 378 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test378::Test378State::Pass,
        "Test 378 reached wrong final state"
    );
}
