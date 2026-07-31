// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test417.scxml:1
use std::time::Duration;

#[test]
fn test_417() {
    let policy = sce_rust_tests::generated::test417::Test417Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 417 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test417::Test417State::Pass,
        "Test 417 reached wrong final state"
    );
}
