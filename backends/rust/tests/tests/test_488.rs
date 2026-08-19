// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 08524b6e9f06ec235417da53ac7c80c6bfd4ac29c2f21bcfec9a9e720a464526
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test488.scxml:1
use std::time::Duration;

#[test]
fn test_488() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test488::Test488Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 488 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test488::Test488State::Pass,
        "Test 488 reached wrong final state"
    );
}
