#[test]
fn fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
    #[cfg(not(nightly_column_names))]
    t.compile_fail("tests/fail-stable/*.rs");
    #[cfg(nightly_column_names)]
    t.compile_fail("tests/fail-nightly/*.rs");
}
