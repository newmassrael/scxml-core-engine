// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2cf4917c7dff79eaf746b52e649909e9c7318e80b65f49555ba6a2bcd0d8eaca
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test436.scxml:1
use std::time::Duration;

#[test]
fn test_436() {
    let policy = sce_rust_tests::generated::test436::Test436Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 436 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test436::Test436State::Pass,
        "Test 436 reached wrong final state"
    );
}
