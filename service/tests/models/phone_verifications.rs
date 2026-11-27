use zerpto_iam::models::phone_verifications::{generate_code, normalize};

#[test]
fn phone_numbers_must_be_e164() {
    assert_eq!(
        normalize("+62 812-3456-7890").as_deref(),
        Some("+6281234567890")
    );
    assert_eq!(normalize("081234567890"), None);
    assert_eq!(normalize("+0123456789"), None);
    assert_eq!(normalize("+62abc"), None);
    assert_eq!(normalize("+123"), None);
}

#[test]
fn codes_are_six_digits() {
    for _ in 0..50 {
        let code = generate_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }
}
