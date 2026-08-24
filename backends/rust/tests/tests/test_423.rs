// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2a328c6a2c55f2d381ea947b66337ce444ad937a90838cfa9cbdecc92a89b987
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test423.scxml:1
use std::time::Duration;

#[test]
fn test_423() {
    let policy = sce_rust_tests::generated::test423::Test423Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 423 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test423::Test423State::Pass,
        "Test 423 reached wrong final state"
    );
}
