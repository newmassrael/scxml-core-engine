// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596474
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test578.scxml:1
use std::time::Duration;

#[test]
fn test_578() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = sce_rust_tests::generated::test578::Test578Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 578 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test578::Test578State::Pass,
        "Test 578 reached wrong final state"
    );
}
