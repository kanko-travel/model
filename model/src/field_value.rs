use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use uuid::Uuid;

use crate::FieldType;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Uuid(Option<Uuid>),
    Bool(Option<bool>),
    Int(Option<i64>),
    Float(Option<f64>),
    Decimal(Option<Decimal>),
    String(Option<String>),
    Date(Option<NaiveDate>),
    DateTime(Option<DateTime<Utc>>),
    Json(Option<Value>),
}

impl FieldValue {
    pub fn field_type(&self) -> FieldType {
        match self {
            Self::Uuid(_) => FieldType::Uuid,
            Self::Bool(_) => FieldType::Bool,
            Self::Int(_) => FieldType::Int,
            Self::Float(_) => FieldType::Float,
            Self::Decimal(_) => FieldType::Decimal,
            Self::String(_) => FieldType::String,
            Self::Date(_) => FieldType::Date,
            Self::DateTime(_) => FieldType::DateTime,
            Self::Json(_) => FieldType::Json,
        }
    }
}

impl ToString for FieldValue {
    fn to_string(&self) -> String {
        match self {
            Self::Uuid(Some(inner)) => inner.to_string(),
            Self::Bool(Some(inner)) => inner.to_string(),
            Self::Int(Some(inner)) => inner.to_string(),
            Self::Float(Some(inner)) => inner.to_string(),
            Self::Decimal(Some(inner)) => inner.to_string(),
            Self::String(Some(inner)) => inner.to_string(),
            Self::Date(Some(inner)) => inner.to_string(),
            Self::DateTime(Some(inner)) => inner.to_rfc3339(),
            Self::Json(Some(inner)) => inner.to_string(),
            _ => "null".to_string(),
        }
    }
}

impl<E> Into<Result<Self, E>> for FieldValue {
    fn into(self) -> Result<Self, E> {
        Ok(self)
    }
}

impl From<Uuid> for FieldValue {
    fn from(value: Uuid) -> Self {
        Self::Uuid(value.into())
    }
}

impl From<bool> for FieldValue {
    fn from(value: bool) -> Self {
        Self::Bool(value.into())
    }
}

impl From<i64> for FieldValue {
    fn from(value: i64) -> Self {
        Self::Int(value.into())
    }
}

impl From<f64> for FieldValue {
    fn from(value: f64) -> Self {
        Self::Float(value.into())
    }
}

impl From<Decimal> for FieldValue {
    fn from(value: Decimal) -> Self {
        Self::Decimal(value.into())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<NaiveDate> for FieldValue {
    fn from(value: NaiveDate) -> Self {
        Self::Date(value.into())
    }
}

impl From<DateTime<Utc>> for FieldValue {
    fn from(value: DateTime<Utc>) -> Self {
        Self::DateTime(value.into())
    }
}

impl From<DateTime<FixedOffset>> for FieldValue {
    fn from(value: DateTime<FixedOffset>) -> Self {
        let value: DateTime<Utc> = value.into();

        Self::DateTime(value.into())
    }
}

impl From<Value> for FieldValue {
    fn from(value: Value) -> Self {
        Self::Json(value.into())
    }
}

impl From<Option<Uuid>> for FieldValue {
    fn from(value: Option<Uuid>) -> Self {
        Self::Uuid(value)
    }
}

impl From<Option<bool>> for FieldValue {
    fn from(value: Option<bool>) -> Self {
        Self::Bool(value)
    }
}

impl From<Option<i64>> for FieldValue {
    fn from(value: Option<i64>) -> Self {
        Self::Int(value)
    }
}

impl From<Option<f64>> for FieldValue {
    fn from(value: Option<f64>) -> Self {
        Self::Float(value)
    }
}

impl From<Option<Decimal>> for FieldValue {
    fn from(value: Option<Decimal>) -> Self {
        Self::Decimal(value)
    }
}

impl From<Option<String>> for FieldValue {
    fn from(value: Option<String>) -> Self {
        Self::String(value)
    }
}

impl From<Option<NaiveDate>> for FieldValue {
    fn from(value: Option<NaiveDate>) -> Self {
        Self::Date(value)
    }
}

impl From<Option<DateTime<Utc>>> for FieldValue {
    fn from(value: Option<DateTime<Utc>>) -> Self {
        Self::DateTime(value)
    }
}

impl From<Option<DateTime<FixedOffset>>> for FieldValue {
    fn from(value: Option<DateTime<FixedOffset>>) -> Self {
        let value: Option<DateTime<Utc>> = if let Some(value) = value {
            Some(value.into())
        } else {
            None
        };

        Self::DateTime(value)
    }
}

impl From<Option<Value>> for FieldValue {
    fn from(value: Option<Value>) -> Self {
        Self::Json(value)
    }
}
