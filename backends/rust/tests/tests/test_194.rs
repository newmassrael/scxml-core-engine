// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c22d767976ad0f3af27597215acac4daa969b18394744727f9f1e4af8f5db2d7
// generated-at: 1785338317
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test194.scxml:1
use std::time::Duration;

#[test]
fn test_194() {
    let policy = sce_rust_tests::generated::test194::Test194Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(completed, "Test 194 timed out");
    assert_eq!(
        engine.get_current_state(),
        sce_rust_tests::generated::test194::Test194State::Pass,
        "Test 194 reached wrong final state"
    );
}
