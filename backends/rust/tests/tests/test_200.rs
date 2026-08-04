// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: bf9f012bd8272e352f46f4d8064cf0cf3b743ab6fffdf8c941cc03f3254cb15f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test200.scxml:1
use std::time::Duration;

#[test]
fn test_200() {
    let policy = sce_rust_tests::generated::test200::Test200Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 200 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test200::Test200State::Pass,
        "Test 200 reached wrong final state"
    );
}
