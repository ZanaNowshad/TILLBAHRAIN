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
    #[error("currency must be a three-letter uppercase ISO code")]
    InvalidCurrency,
    #[error("currency exponent must be between 0 and 6")]
    InvalidExponent,
    #[error("money currency/exponent mismatch")]
    CurrencyMismatch,
    #[error("money arithmetic overflow")]
    Overflow,
}

impl Money {
    pub fn new(minor: i64, currency: impl Into<String>, exponent: u8) -> Result<Self, MoneyError> {
        let currency = currency.into();
        if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_uppercase()) {
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

    pub fn checked_add(&self, rhs: Money) -> Result<Self, MoneyError> {
        self.ensure_compatible(&rhs)?;
        let minor = self.minor.checked_add(rhs.minor).ok_or(MoneyError::Overflow)?;
        Self::new(minor, self.currency.clone(), self.exponent)
    }

    pub fn checked_sub(&self, rhs: Money) -> Result<Self, MoneyError> {
        self.ensure_compatible(&rhs)?;
        let minor = self.minor.checked_sub(rhs.minor).ok_or(MoneyError::Overflow)?;
        Self::new(minor, self.currency.clone(), self.exponent)
    }

    pub fn to_major_string(&self) -> String {
        if self.exponent == 0 {
            return self.minor.to_string();
        }
        let negative = self.minor < 0;
        let abs = i128::from(self.minor).abs();
        let factor = 10_i128.pow(u32::from(self.exponent));
        let major = abs / factor;
        let fraction = abs % factor;
        format!(
            "{}{}.{:0width$}",
            if negative { "-" } else { "" },
            major,
            fraction,
            width = usize::from(self.exponent)
        )
    }

    fn ensure_compatible(&self, rhs: &Self) -> Result<(), MoneyError> {
        if self.currency != rhs.currency || self.exponent != rhs.exponent {
            return Err(MoneyError::CurrencyMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_negative_min_value_without_overflow() {
        let amount = Money::bhd(i64::MIN);
        assert!(amount.to_major_string().starts_with('-'));
    }
}
