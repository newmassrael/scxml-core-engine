// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 392bbcde4466dbc0cb9cb0e8b35901796c2cabcfe17ca0552a2f1bf1fe87d8de
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test280.scxml:1
use std::time::Duration;

#[test]
fn test_280() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test280::Test280Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 280 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test280::Test280State::Pass,
        "Test 280 reached wrong final state"
    );
}
