use rust_decimal::Decimal;
use std::str::FromStr;
use tillbahrain_lib::domain::{Money, Quantity};

#[test]
fn bhd_money_uses_exact_minor_units() {
    let amount = Money::new(12_345, "BHD", 3).expect("valid BHD amount");
    assert_eq!(amount.minor(), 12_345);
    assert_eq!(amount.currency(), "BHD");
    assert_eq!(amount.exponent(), 3);
    assert_eq!(amount.to_major_string(), "12.345");
}

#[test]
fn money_rejects_invalid_currency_and_exponent() {
    assert!(Money::new(1000, "bd", 3).is_err());
    assert!(Money::new(1000, "BHD", 10).is_err());
}

#[test]
fn money_addition_requires_same_currency_and_exponent() {
    let a = Money::new(1_000, "BHD", 3).unwrap();
    let b = Money::new(250, "BHD", 3).unwrap();
    let c = Money::new(100, "USD", 2).unwrap();
    assert_eq!(a.checked_add(b).unwrap().minor(), 1_250);
    assert!(a.checked_add(c).is_err());
}

#[test]
fn quantity_is_decimal_not_float() {
    let quantity = Quantity::from_str("1.250").expect("valid exact quantity");
    assert_eq!(quantity.decimal(), Decimal::from_str("1.250").unwrap());
    assert_eq!(quantity.normalized_string(), "1.25");
}

#[test]
fn quantity_rejects_zero_negative_and_excess_scale() {
    assert!(Quantity::from_str("0").is_err());
    assert!(Quantity::from_str("-1").is_err());
    assert!(Quantity::from_str("0.000001").is_err());
}
