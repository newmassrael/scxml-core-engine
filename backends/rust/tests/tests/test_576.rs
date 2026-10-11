// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
use std::time::Duration;

#[test]
fn test_576() {
    let policy = sce_rust_tests::generated::test576::Test576Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 576 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test576::Test576State::Pass),
        "Test 576 reached wrong final state"
    );
}
