// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 128f5bda1db8a8695e204b38e87b8d2d3815bdde9691186823a5ecdc7374af1d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test409.scxml:1
use std::time::Duration;

#[test]
fn test_409() {
    let policy = sce_rust_tests::generated::test409::Test409Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(5), Duration::from_millis(10));
    assert!(completed, "Test 409 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test409::Test409State::Pass,
        "Test 409 reached wrong final state"
    );
}
