// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test346.scxml:1
use std::time::Duration;

#[test]
fn test_346() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test346::Test346Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 346 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test346::Test346State::Pass,
        "Test 346 reached wrong final state"
    );
}
