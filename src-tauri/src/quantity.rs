use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

const MAX_QUANTITY_SCALE: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Quantity(Decimal);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QuantityError {
    #[error("quantity is not a valid decimal")]
    InvalidDecimal,
    #[error("quantity precision exceeds six decimal places")]
    ExcessiveScale,
    #[error("quantity must be greater than zero")]
    NonPositive,
    #[error("quantity arithmetic overflow")]
    Overflow,
}

impl Quantity {
    pub fn parse(value: &str) -> Result<Self, QuantityError> {
        let decimal = Decimal::from_str(value).map_err(|_| QuantityError::InvalidDecimal)?;
        Self::new(decimal)
    }

    pub fn new(value: Decimal) -> Result<Self, QuantityError> {
        if value.scale() > MAX_QUANTITY_SCALE {
            return Err(QuantityError::ExcessiveScale);
        }
        Ok(Self(value.normalize()))
    }

    pub fn decimal(self) -> Decimal {
        self.0
    }

    pub fn require_positive(self) -> Result<Self, QuantityError> {
        if self.0 > Decimal::ZERO {
            Ok(self)
        } else {
            Err(QuantityError::NonPositive)
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self, QuantityError> {
        let value = self.0.checked_add(other.0).ok_or(QuantityError::Overflow)?;
        Self::new(value)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, QuantityError> {
        let value = self.0.checked_sub(other.0).ok_or(QuantityError::Overflow)?;
        Self::new(value)
    }
}

impl std::fmt::Display for Quantity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_quantity_is_exact() {
        let one_tenth = Quantity::parse("0.1").unwrap();
        let two_tenths = Quantity::parse("0.2").unwrap();
        assert_eq!(one_tenth.checked_add(two_tenths).unwrap().to_string(), "0.3");
    }

    #[test]
    fn inventory_delta_can_be_negative() {
        assert_eq!(Quantity::parse("-2.500").unwrap().to_string(), "-2.5");
    }

    #[test]
    fn sale_quantity_can_require_positive_value() {
        assert_eq!(
            Quantity::parse("0").unwrap().require_positive(),
            Err(QuantityError::NonPositive)
        );
        assert!(Quantity::parse("0.125").unwrap().require_positive().is_ok());
    }

    #[test]
    fn excessive_scale_is_rejected() {
        assert_eq!(
            Quantity::parse("0.0000001"),
            Err(QuantityError::ExcessiveScale)
        );
    }
}
