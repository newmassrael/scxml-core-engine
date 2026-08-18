// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 123759fa1515134527b83cfd094acff4a38d0e67d776745e7939fe5a5955e20a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test412.scxml:1
use std::time::Duration;

#[test]
fn test_412() {
    let policy = sce_rust_tests::generated::test412::Test412Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 412 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test412::Test412State::Pass,
        "Test 412 reached wrong final state"
    );
}
