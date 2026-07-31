// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486330
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test521.scxml:1
use std::time::Duration;

#[test]
fn test_521() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test521::Test521Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 521 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test521::Test521State::Pass,
        "Test 521 reached wrong final state"
    );
}
