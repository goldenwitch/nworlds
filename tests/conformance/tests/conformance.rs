use caravan_conformance::cases;

#[test]
fn every_catalogued_conformance_case_passes() {
    for case in cases() {
        (case.run)();
    }
}
