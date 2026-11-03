// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test185.scxml:1
use std::time::Duration;

#[test]
fn test_185() {
    let policy = sce_rust_tests::generated::test185::Test185Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 185 timed out");
    assert_eq!(
        engine.terminal_state(),
        Some(sce_rust_tests::generated::test185::Test185State::Pass),
        "Test 185 reached wrong final state"
    );
}
