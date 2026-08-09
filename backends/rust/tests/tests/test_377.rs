// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 392bbcde4466dbc0cb9cb0e8b35901796c2cabcfe17ca0552a2f1bf1fe87d8de
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test377.scxml:1
use std::time::Duration;

#[test]
fn test_377() {
    let policy = sce_rust_tests::generated::test377::Test377Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 377 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test377::Test377State::Pass,
        "Test 377 reached wrong final state"
    );
}
