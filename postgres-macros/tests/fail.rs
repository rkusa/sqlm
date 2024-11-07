#[test]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
    #[cfg(not(nightly_fail_tests))]
    t.compile_fail("tests/fail-stable/*.rs");
    #[cfg(nightly_fail_tests)]
    t.compile_fail("tests/fail-nightly/*.rs");
}
