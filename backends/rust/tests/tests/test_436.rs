// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
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
