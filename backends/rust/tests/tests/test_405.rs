// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test405.scxml:1
use std::time::Duration;

#[test]
fn test_405() {
    let policy = sce_rust_tests::generated::test405::Test405Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 405 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test405::Test405State::Pass,
        "Test 405 reached wrong final state"
    );
}
