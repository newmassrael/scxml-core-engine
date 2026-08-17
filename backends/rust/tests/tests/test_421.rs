// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test421.scxml:1
use std::time::Duration;

#[test]
fn test_421() {
    let policy = sce_rust_tests::generated::test421::Test421Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 421 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test421::Test421State::Pass,
        "Test 421 reached wrong final state"
    );
}
