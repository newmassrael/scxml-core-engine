// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7d180dffdd955c10062343fb76305c7a80a95112d21da2591e0f0959805b08ad
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test232.scxml:1
use std::time::Duration;

#[test]
fn test_232() {
    let policy = sce_rust_tests::generated::test232::Test232Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 232 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test232::Test232State::Pass,
        "Test 232 reached wrong final state"
    );
}
