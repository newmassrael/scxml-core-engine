// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test404.scxml:1
use std::time::Duration;

#[test]
fn test_404() {
    let policy = sce_rust_tests::generated::test404::Test404Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 404 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test404::Test404State::Pass,
        "Test 404 reached wrong final state"
    );
}
