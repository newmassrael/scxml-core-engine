// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 025e57d78939dcd3c3bbc54b606a62c00b45f367a9a3d9faa2cdd4bf5896d8fc
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test311.scxml:1
use std::time::Duration;

#[test]
fn test_311() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test311::Test311Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 311 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test311::Test311State::Pass,
        "Test 311 reached wrong final state"
    );
}
