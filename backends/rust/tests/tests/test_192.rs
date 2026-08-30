// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 057f3064c2c620977191e86f67c1d505edec850a0d81b50b27d4b101952af703
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test192.scxml:1
use std::time::Duration;

#[test]
fn test_192() {
    let policy = sce_rust_tests::generated::test192::Test192Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 192 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test192::Test192State::Pass,
        "Test 192 reached wrong final state"
    );
}
