use zerpto_iam::models::address::{Address, AddressInput, Resolved};

fn typed() -> AddressInput {
    AddressInput {
        region_id: None,
        line1: Some("Jl. Kebon Sirih No. 10".to_string()),
        line2: None,
        country: Some("id".to_string()),
        province: Some("typed province".to_string()),
        district: Some("typed district".to_string()),
        sub_district: None,
        village: Some("Kebon Sirih".to_string()),
        postal_code: Some("99999".to_string()),
    }
}

#[test]
fn the_region_replaces_the_levels_it_covers() {
    let address = Address::complete(
        typed(),
        Resolved {
            country: "ID".to_string(),
            province: "DKI Jakarta".to_string(),
            district: "Kota Administrasi Jakarta Pusat".to_string(),
            sub_district: "Menteng".to_string(),
            village: String::new(),
            postal_code: "10310".to_string(),
        },
    );
    assert_eq!(address.country.as_deref(), Some("ID"));
    assert_eq!(address.province.as_deref(), Some("DKI Jakarta"));
    assert_eq!(
        address.district.as_deref(),
        Some("Kota Administrasi Jakarta Pusat")
    );
    assert_eq!(address.sub_district.as_deref(), Some("Menteng"));
    assert_eq!(address.postal_code.as_deref(), Some("10310"));
}

#[test]
fn a_level_below_the_tree_stays_as_typed() {
    let address = Address::complete(
        typed(),
        Resolved {
            country: "ID".to_string(),
            ..Resolved::default()
        },
    );
    assert_eq!(address.village.as_deref(), Some("Kebon Sirih"));
}

#[test]
fn a_typed_postal_code_stays_when_no_region_has_one() {
    let address = Address::complete(
        typed(),
        Resolved {
            country: "ID".to_string(),
            province: "DKI Jakarta".to_string(),
            ..Resolved::default()
        },
    );
    assert_eq!(address.postal_code.as_deref(), Some("99999"));
}

#[test]
fn a_hand_typed_address_keeps_every_typed_field_and_upper_cases_the_country() {
    let address = Address::complete(typed(), Resolved::default());
    assert_eq!(address.country.as_deref(), Some("ID"));
    assert_eq!(address.province.as_deref(), Some("typed province"));
    assert_eq!(address.district.as_deref(), Some("typed district"));
    assert_eq!(address.line1.as_deref(), Some("Jl. Kebon Sirih No. 10"));
}
