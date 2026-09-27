// SCE-GENERATED — DO NOT EDIT
// source-hash: c004aa6d76072d31e9273eed0dc88dc847e351ce7e35f90cc9d56908f01328bd
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
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test192::Test192State::Pass),
        "Test 192 reached wrong final state"
    );
}
