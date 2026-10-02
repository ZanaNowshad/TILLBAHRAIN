use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

const MAX_SCALE: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity(Decimal);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum QuantityError {
    #[error("invalid decimal quantity")]
    Invalid,
    #[error("quantity must be greater than zero")]
    NotPositive,
    #[error("quantity precision exceeds supported scale")]
    ExcessScale,
}

impl Quantity {
    pub fn new(value: Decimal) -> Result<Self, QuantityError> {
        if value <= Decimal::ZERO {
            return Err(QuantityError::NotPositive);
        }
        if value.scale() > MAX_SCALE {
            return Err(QuantityError::ExcessScale);
        }
        Ok(Self(value))
    }

    pub fn decimal(self) -> Decimal {
        self.0
    }

    pub fn normalized_string(self) -> String {
        self.0.normalize().to_string()
    }
}

impl FromStr for Quantity {
    type Err = QuantityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let decimal = Decimal::from_str(value).map_err(|_| QuantityError::Invalid)?;
        Self::new(decimal)
    }
}
