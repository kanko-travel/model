use std::collections::HashMap;
use uuid::Uuid;

use crate::relation::RelationDef;
use crate::Error;
use crate::FieldValue;

pub trait Model {
    fn table_name() -> String;
    fn id_field_name() -> String;
    fn field_definitions() -> Vec<FieldDefinition>;
    fn definition() -> ModelDef {
        ModelDef {
            table_name: Self::table_name,
            id_field_name: Self::id_field_name,
            field_definitions: Self::field_definitions,
        }
    }

    fn belongs_to<R>(name: String, column: String) -> RelationDef
    where
        Self: Sized,
        R: Model,
    {
        RelationDef::belongs_to::<Self, R>(name, column)
    }

    fn has_one<R>(name: String, column: String) -> RelationDef
    where
        Self: Sized,
        R: Model,
    {
        RelationDef::has_one::<Self, R>(name, column)
    }

    fn has_many<R>(name: String, column: String) -> RelationDef
    where
        Self: Sized,
        R: Model,
    {
        RelationDef::has_many::<Self, R>(name, column)
    }

    fn has_many_via_junction_table<R>(name: String, junction_table_name: String) -> RelationDef
    where
        Self: Sized,
        R: Model,
    {
        RelationDef::has_many_via_junction_table::<Self, R>(name, junction_table_name)
    }

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

pub struct ModelDef {
    pub table_name: fn() -> String,
    pub id_field_name: fn() -> String,
    pub field_definitions: fn() -> Vec<FieldDefinition>,
}

pub trait Enum: Sized {
    fn try_from_string(value: String) -> Result<Self, Error>;
    fn to_string(self) -> String;

    fn variants() -> Vec<String>;
}

#[derive(Clone, Debug, PartialEq)]
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
    Int32,
    Float,
    Decimal,
    String,
    Date,
    DateTime,
    Json,
    Enum(Vec<String>),
}

impl FieldType {
    pub fn null_value(&self) -> FieldValue {
        match self {
            Self::Uuid => FieldValue::Uuid(None),
            Self::Bool => FieldValue::Bool(None),
            Self::Int => FieldValue::Int(None),
            Self::Int32 => FieldValue::Int32(None),
            Self::Float => FieldValue::Float(None),
            Self::Decimal => FieldValue::Decimal(None),
            Self::String => FieldValue::String(None),
            Self::Date => FieldValue::Date(None),
            Self::DateTime => FieldValue::DateTime(None),
            Self::Json => FieldValue::Json(None),
            Self::Enum(_) => FieldValue::Enum(None),
        }
    }

    pub fn sql_type(&self) -> &'static str {
        match self {
            Self::Uuid => "uuid",
            Self::Bool => "boolean",
            Self::Int => "int8",
            Self::Int32 => "int4",
            Self::Float => "float8",
            Self::Decimal => "decimal",
            Self::String => "text",
            Self::Date => "date",
            Self::DateTime => "timestamptz",
            Self::Json => "jsonb",
            Self::Enum(_) => "text",
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

#[cfg(test)]
mod tests {
    use crate as model;
    use model::{Enum, Model};
    use serde::{Deserialize, Serialize};
    use sqlx::prelude::FromRow;
    use uuid::Uuid;

    #[test]
    fn test_model_with_enum() {
        #[derive(Clone, Serialize, Deserialize, Enum)]
        enum TestEnum {
            On,
            Off,
        }

        #[derive(Serialize, Deserialize, FromRow, Model)]
        #[model(table_name = "test_model")]
        struct TestModel {
            #[model(primary_key, id)]
            id: Uuid,
            #[model(enum)]
            test_enum: TestEnum,
        }

        let on = TestEnum::On;

        let on_string = on.to_string();
        assert_eq!(on_string, "On".to_string());

        TestEnum::try_from_string(on_string).unwrap();
    }
}
