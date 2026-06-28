#[test]
#[ignore]
fn ui_tests() {
    let t = trybuild::TestCases::new();

    t.compile_fail("tests/ui/postgres/*.rs");

    // UI tests for column types that require gated features
    if cfg!(not(feature = "chrono")) && cfg!(not(feature = "time")) {
        t.compile_fail("tests/ui/postgres/gated/chrono.rs");
    }

    if cfg!(not(feature = "uuid")) {
        t.compile_fail("tests/ui/postgres/gated/uuid.rs");
    }

    if cfg!(not(feature = "ipnet")) && cfg!(not(feature = "ipnetwork")) {
        t.compile_fail("tests/ui/postgres/gated/ipnetwork.rs");
    }

    t.compile_fail("tests/ui/*.rs");
}
