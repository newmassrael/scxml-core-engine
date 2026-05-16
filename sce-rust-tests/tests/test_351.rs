// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932418
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test351.scxml:1
use std::time::Duration;

#[test]
fn test_351() {
    let _ = sce_rust_lua::register();
    let policy = sce_rust_tests::generated::test351::Test351Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(
        Duration::from_secs(3),
        Duration::from_millis(10),
    );
    assert!(completed, "Test 351 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test351::Test351State::Pass,
        "Test 351 reached wrong final state"
    );
}
