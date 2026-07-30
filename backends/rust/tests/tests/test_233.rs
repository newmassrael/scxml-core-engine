// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425169
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test233.scxml:1
use std::time::Duration;

#[test]
fn test_233() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test233::Test233Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 233 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test233::Test233State::Pass,
        "Test 233 reached wrong final state"
    );
}
