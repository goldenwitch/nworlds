#![forbid(unsafe_code)]

#[test]
fn public_boundary_rejects_purity_escapes() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
