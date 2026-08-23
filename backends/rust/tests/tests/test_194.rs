// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 90a17b01a07fa6e2db248839e7823280de5374963642bfa559a2f033a153c586
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test194.scxml:1
use std::time::Duration;

#[test]
fn test_194() {
    let policy = sce_rust_tests::generated::test194::Test194Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 194 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test194::Test194State::Pass,
        "Test 194 reached wrong final state"
    );
}
