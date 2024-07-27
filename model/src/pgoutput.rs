use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{Error, FieldDefinitionMap, FieldType, FieldValue, Model};

pub trait FromPgoutput {
    fn from_pgoutput(col_names: &Vec<String>, row: Vec<Option<String>>) -> Result<Self, Error>
    where
        Self: Sized;
}

impl<T> FromPgoutput for T
where
    T: Model + for<'de> Deserialize<'de>,
{
    fn from_pgoutput(col_names: &Vec<String>, row: Vec<Option<String>>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if row.len() != col_names.len() {
            return Err(Error::bad_request(
                "column names and values must have the same length",
            ));
        }

        let field_defs: FieldDefinitionMap = Self::field_definitions().into();

        let mut map = HashMap::new();

        for (i, column_name) in col_names.iter().enumerate() {
            let def = field_defs
                .0
                .get(column_name)
                .ok_or_else(|| Error::bad_request("undefined column present in pgoutput"))?;

            let value = row.get(i).unwrap();

            // decode the value into a serde_json::Value
            let value = if let Some(val) = value {
                match &def.type_ {
                    FieldType::Uuid | FieldType::Reference(_) => Uuid::parse_str(&val)
                        .map_err(|_| Error::bad_request("invalid uuid"))?
                        .into(),
                    FieldType::Bool => bool::from_str(&val)
                        .map_err(|_| Error::bad_request("invalid bool"))?
                        .into(),
                    FieldType::Int => i64::from_str(&val)
                        .map_err(|_| Error::bad_request("invalid int"))?
                        .into(),
                    FieldType::Int32 => i32::from_str(&val)
                        .map_err(|_| Error::bad_request("invalid int"))?
                        .into(),
                    FieldType::Float => f64::from_str(&val)
                        .map_err(|_| Error::bad_request("invalid message"))?
                        .into(),
                    FieldType::Decimal => Decimal::from_str(&val)
                        .map_err(|_| Error::bad_request("invalid decimal"))?
                        .into(),
                    FieldType::String => val.clone().into(),
                    FieldType::Date => NaiveDate::parse_from_str(&val, "%Y-%m-%d")
                        .map_err(|_| Error::bad_request("invalid date"))?
                        .into(),
                    FieldType::DateTime => DateTime::parse_from_rfc3339(&val)
                        .map_err(|_| Error::bad_request("invalid datetime"))?
                        .into(),
                    FieldType::Enum(variants) => {
                        variants
                            .iter()
                            .find(|&v| v == val)
                            .ok_or_else(|| Error::bad_request("invalid enum variant"))?;

                        FieldValue::Enum(val.to_string().into())
                    }
                    FieldType::Json => {
                        let val: Value = serde_json::from_str(val)
                            .map_err(|_| Error::bad_request("invalid json"))?;

                        FieldValue::Json(val.into())
                    }
                }
            } else {
                def.type_.null_value()
            };

            map.insert(column_name, json!(value));
        }

        println!("{:?}", map);

        let t = serde_json::from_value(json!(map)).map_err(|err| {
            Error::bad_request(&format!(
                "couldn't deserialize pgoutput into expected type: {:?}",
                err
            ))
        })?;

        Ok(t)
    }
}
