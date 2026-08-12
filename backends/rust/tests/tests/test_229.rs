// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 74ba562b33766da248288b5dadec1e79a0ebb46a66e38786f6a7a4b2ccd653e3
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test229.scxml:1
use std::time::Duration;

#[test]
fn test_229() {
    let policy = sce_rust_tests::generated::test229::Test229Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 229 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test229::Test229State::Pass,
        "Test 229 reached wrong final state"
    );
}
