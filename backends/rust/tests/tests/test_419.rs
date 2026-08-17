// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c368ce80174f466d84e6185a3a865287545abdfeafb6bd04a27d03c8ef959c7a
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test419.scxml:1
use std::time::Duration;

#[test]
fn test_419() {
    let policy = sce_rust_tests::generated::test419::Test419Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 419 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test419::Test419State::Pass,
        "Test 419 reached wrong final state"
    );
}
