// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 94a6daf42142517c0ee9ba49a95e1db9d84d30a097beabea83a903e1d7ba88bf
// generated-at: 1779020074
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test567.scxml:1
use std::time::Duration;

#[test]
fn test_567() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test567::Test567Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 567 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test567::Test567State::Pass,
        "Test 567 reached wrong final state"
    );
}
