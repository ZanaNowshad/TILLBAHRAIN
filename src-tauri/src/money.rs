use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    minor: i64,
    currency: String,
    exponent: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("currency must be a three-letter ASCII code")]
    InvalidCurrency,
    #[error("currency exponent must be between 0 and 6")]
    InvalidExponent,
    #[error("money operands use different currencies or exponents")]
    CurrencyMismatch,
    #[error("money arithmetic overflow")]
    Overflow,
}

impl Money {
    pub fn new(minor: i64, currency: impl Into<String>, exponent: u8) -> Result<Self, MoneyError> {
        let currency = currency.into().to_ascii_uppercase();
        if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err(MoneyError::InvalidCurrency);
        }
        if exponent > 6 {
            return Err(MoneyError::InvalidExponent);
        }
        Ok(Self {
            minor,
            currency,
            exponent,
        })
    }

    pub fn bhd(minor: i64) -> Self {
        Self {
            minor,
            currency: "BHD".to_owned(),
            exponent: 3,
        }
    }

    pub fn minor(&self) -> i64 {
        self.minor
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }

    pub fn exponent(&self) -> u8 {
        self.exponent
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, MoneyError> {
        self.ensure_same_unit(other)?;
        let minor = self.minor.checked_add(other.minor).ok_or(MoneyError::Overflow)?;
        Self::new(minor, self.currency.clone(), self.exponent)
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, MoneyError> {
        self.ensure_same_unit(other)?;
        let minor = self.minor.checked_sub(other.minor).ok_or(MoneyError::Overflow)?;
        Self::new(minor, self.currency.clone(), self.exponent)
    }

    pub fn format_amount(&self) -> String {
        if self.exponent == 0 {
            return self.minor.to_string();
        }
        let factor = 10_i128.pow(u32::from(self.exponent));
        let value = i128::from(self.minor);
        let sign = if value < 0 { "-" } else { "" };
        let absolute = value.abs();
        let whole = absolute / factor;
        let fraction = absolute % factor;
        format!(
            "{sign}{whole}.{fraction:0width$}",
            width = usize::from(self.exponent)
        )
    }

    fn ensure_same_unit(&self, other: &Self) -> Result<(), MoneyError> {
        if self.currency == other.currency && self.exponent == other.exponent {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bhd_is_three_decimal_minor_units() {
        let amount = Money::bhd(12_345);
        assert_eq!(amount.currency(), "BHD");
        assert_eq!(amount.exponent(), 3);
        assert_eq!(amount.format_amount(), "12.345");
    }

    #[test]
    fn formats_negative_values_without_float_conversion() {
        assert_eq!(Money::bhd(-5).format_amount(), "-0.005");
        assert_eq!(Money::bhd(i64::MIN).format_amount(), "-9223372036854775.808");
    }

    #[test]
    fn rejects_cross_currency_arithmetic() {
        let bhd = Money::new(1_000, "BHD", 3).unwrap();
        let kwd = Money::new(1_000, "KWD", 3).unwrap();
        assert_eq!(bhd.checked_add(&kwd), Err(MoneyError::CurrencyMismatch));
    }

    #[test]
    fn detects_overflow() {
        let max = Money::bhd(i64::MAX);
        assert_eq!(max.checked_add(&Money::bhd(1)), Err(MoneyError::Overflow));
    }
}
