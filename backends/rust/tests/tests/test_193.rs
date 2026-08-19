// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: daab006ae7e0eecfbb4178e7ccbc8a9f062b3756932cf56951788a953474a874
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test193.scxml:1
use std::time::Duration;

#[test]
fn test_193() {
    let policy = sce_rust_tests::generated::test193::Test193Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 193 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test193::Test193State::Pass,
        "Test 193 reached wrong final state"
    );
}
