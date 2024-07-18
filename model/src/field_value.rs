use std::cmp::Ordering;

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::Enum;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Uuid(Option<Uuid>),
    Bool(Option<bool>),
    Int(Option<i64>),
    Int32(Option<i32>),
    Float(Option<f64>),
    Decimal(Option<Decimal>),
    String(Option<String>),
    Date(Option<NaiveDate>),
    DateTime(Option<DateTime<Utc>>),
    Json(Option<Value>),
    Enum(Option<String>),
}

impl PartialOrd for FieldValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Uuid(a), Self::Uuid(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Bool(a), Self::Bool(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Int(a), Self::Int(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Int32(a), Self::Int32(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Float(a), Self::Float(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Decimal(a), Self::Decimal(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::String(a), Self::String(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Date(a), Self::Date(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::DateTime(a), Self::DateTime(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            (Self::Enum(a), Self::Enum(b)) => match (a, b) {
                (Some(a), Some(b)) => a.partial_cmp(b),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
                (None, None) => Some(Ordering::Equal),
            },
            _ => None,
        }
    }
}

impl ToString for FieldValue {
    fn to_string(&self) -> String {
        match self {
            Self::Uuid(Some(inner)) => inner.to_string(),
            Self::Bool(Some(inner)) => inner.to_string(),
            Self::Int(Some(inner)) => inner.to_string(),
            Self::Int32(Some(inner)) => inner.to_string(),
            Self::Float(Some(inner)) => inner.to_string(),
            Self::Decimal(Some(inner)) => inner.to_string(),
            Self::String(Some(inner)) => inner.to_string(),
            Self::Date(Some(inner)) => inner.to_string(),
            Self::DateTime(Some(inner)) => inner.to_rfc3339(),
            Self::Json(Some(inner)) => inner.to_string(),
            Self::Enum(Some(inner)) => inner.to_string(),
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

impl From<i32> for FieldValue {
    fn from(value: i32) -> Self {
        Self::Int32(value.into())
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

impl<T> From<T> for FieldValue
where
    T: Enum,
{
    fn from(value: T) -> Self {
        Self::Enum(value.to_string().into())
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

impl From<Option<i32>> for FieldValue {
    fn from(value: Option<i32>) -> Self {
        Self::Int32(value)
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

impl<T> From<Option<T>> for FieldValue
where
    T: Enum,
{
    fn from(value: Option<T>) -> Self {
        Self::Enum(value.map(|v| v.to_string()))
    }
}
