// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480866
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
use std::time::Duration;

#[test]
fn test_577() {
    let policy = sce_rust_tests::generated::test577::Test577Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 577 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test577::Test577State::Pass,
        "Test 577 reached wrong final state"
    );
}
