use std::collections::HashMap;
use uuid::Uuid;

use crate::Error;
use crate::FieldValue;

pub trait Model {
    fn table_name() -> String;
    fn id_field_name() -> String;
    fn field_definitions() -> Vec<FieldDefinition>;
    fn id_field_value(&self) -> Uuid;
    fn field_value(&self, field: &str) -> Result<FieldValue, Error>;
    fn fields(&self) -> Result<Vec<(FieldDefinition, FieldValue)>, Error> {
        let defs = Self::field_definitions();
        let mut fields = vec![];

        for def in defs.into_iter() {
            let field_name = def.name.clone();
            fields.push((def, self.field_value(&field_name)?));
        }

        Ok(fields)
    }
}

#[derive(Clone, Debug)]
pub struct FieldDefinition {
    pub name: String,
    pub type_: FieldType,
    pub immutable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub nullable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    Uuid,
    Bool,
    Int,
    Float,
    Decimal,
    String,
    Date,
    DateTime,
    Json,
}

impl FieldType {
    pub fn null_value(&self) -> FieldValue {
        match self {
            Self::Uuid => FieldValue::Uuid(None),
            Self::Bool => FieldValue::Bool(None),
            Self::Int => FieldValue::Int(None),
            Self::Float => FieldValue::Float(None),
            Self::Decimal => FieldValue::Decimal(None),
            Self::String => FieldValue::String(None),
            Self::Date => FieldValue::Date(None),
            Self::DateTime => FieldValue::DateTime(None),
            Self::Json => FieldValue::Json(None),
        }
    }

    pub fn sql_type(&self) -> &'static str {
        match self {
            Self::Uuid => "uuid",
            Self::Bool => "boolean",
            Self::Int => "int8",
            Self::Float => "float8",
            Self::Decimal => "decimal",
            Self::String => "text",
            Self::Date => "date",
            Self::DateTime => "timestamptz",
            Self::Json => "jsonb",
        }
    }
}

#[derive(Clone)]
pub(crate) struct FieldDefinitionMap(pub HashMap<String, FieldDefinition>);

impl From<Vec<FieldDefinition>> for FieldDefinitionMap {
    fn from(value: Vec<FieldDefinition>) -> Self {
        FieldDefinitionMap(value.into_iter().map(|v| (v.name.clone(), v)).collect())
    }
}
