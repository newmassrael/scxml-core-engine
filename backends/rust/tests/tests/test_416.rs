// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test416.scxml:1
use std::time::Duration;

#[test]
fn test_416() {
    let policy = sce_rust_tests::generated::test416::Test416Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 416 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test416::Test416State::Pass),
        "Test 416 reached wrong final state"
    );
}
