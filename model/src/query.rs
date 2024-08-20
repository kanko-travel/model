use std::borrow::Cow;
use std::marker::PhantomData;

use crate::cursor::Cursor;
use crate::field_value::FieldValue;
use crate::filter::ast::{Expr, Var};
use crate::model::FieldType;
use crate::util::from_b64_str;
use crate::Model;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
pub use serde_json;
use uuid::Uuid;

use crate::Error;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct RawQuery {
    pub filter: Option<String>,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct Query<T: Model> {
    pub filter: Option<Expr>,
    pub sort: Option<Sort>,
    pub cursor: Option<Cursor>,
    pub limit: Option<i64>,

    // internal fields
    _marker: PhantomData<T>,
}

#[derive(Clone, Debug)]
pub struct Sort {
    pub field: Var,
    pub direction: SortDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

impl<T: Model> Query<T> {
    pub fn new() -> Self {
        Self {
            filter: None,
            sort: None,
            cursor: None,
            limit: None,
            _marker: PhantomData,
        }
    }
}

impl<'de, T: Model> Deserialize<'de> for Query<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_query = RawQuery::deserialize(deserializer)?;

        // debugging
        println!("{:?}", raw_query);

        let query: Query<T> = raw_query.try_into().map_err(serde::de::Error::custom)?;

        // debugging
        println!("successfully parsed query");
        let sql = query.filter.as_ref().map(|f| {
            let (sql, _, _) = f.to_sql::<T>(0);
            sql
        });
        println!("filter: {:?}", sql);

        Ok(query)
    }
}

impl<T: Model + JsonSchema> JsonSchema for Query<T> {
    fn schema_name() -> String {
        // Exclude the module path to make the name in generated schemas clearer.
        // "Query".to_owned()
        format!("{}Query", T::schema_name())
    }

    fn schema_id() -> Cow<'static, str> {
        // Include the module, in case a type with the same name is in another module/crate
        // Cow::Borrowed(concat!(module_path!(), "::Query"))
        Cow::Owned(format!("{}::Query<{}>", module_path!(), T::schema_id()))
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        RawQuery::json_schema(gen)
    }
}

impl<T: Model> TryFrom<RawQuery> for Query<T> {
    type Error = Error;

    fn try_from(value: RawQuery) -> Result<Self, Self::Error> {
        let mut query = Query::new();
        let model_def = T::definition();

        if let Some(filter) = value.filter {
            query.filter = parse_filter::<T>(&filter)?.into();
        }

        if let Some(sort_by) = value.sort_by {
            let field = Var::from_sort_by_str::<T>(&sort_by)?;
            let field_def = field.resolve_definition(&model_def)?;

            if let FieldType::Json = field_def.type_ {
                return Err(Error::bad_request(
                    "invalid sort_by field: field is not sortable",
                ));
            }

            let mut sort = Sort {
                field,
                // this is the default sort direction
                direction: SortDirection::Ascending,
            };

            // parse the sort direction if it exists
            if let Some(sort_direction) = value.sort_direction {
                let sort_direction = parse_sort_direction(&sort_direction)?;

                sort.direction = sort_direction;
            }

            query.sort = sort.into();

            // parse the cursor if it exists
            if let Some(cursor) = value.cursor {
                let cursor_value = if sort_by == T::id_field_name() {
                    None
                } else {
                    (&field_def.type_).into()
                };

                let cursor = parse_cursor(&cursor, cursor_value)?;
                query.cursor = cursor.into();
            }
        } else if let Some(cursor) = value.cursor {
            let cursor = parse_cursor(&cursor, None)?;
            query.cursor = cursor.into();
        }

        query.limit = value.limit;

        Ok(query)
    }
}

fn parse_filter<T: Model>(filter: &str) -> Result<Expr, Error> {
    Expr::from_str::<T>(filter)
}

fn parse_sort_direction(sort_direction: &str) -> Result<SortDirection, Error> {
    match sort_direction {
        "1" => Ok(SortDirection::Ascending),
        "-1" => Ok(SortDirection::Descending),
        _ => Err(Error::bad_request("sort direction must be one of 1 or -1")),
    }
}

fn parse_cursor(cursor: &str, value_type: Option<&FieldType>) -> Result<Cursor, Error> {
    let mut parts = cursor.split('_');

    let id = parts
        .next()
        .ok_or_else(|| Error::bad_request("invalid cursor"))?;

    let id: Uuid = from_b64_str(id)?;

    let value = if let Some(value_type) = value_type {
        let value = parts
            .next()
            .ok_or_else(|| Error::bad_request("invalid cursor: expected value component"))?;

        match value_type {
            FieldType::Uuid => FieldValue::Uuid(from_b64_str(value)?),
            FieldType::Bool => FieldValue::Bool(from_b64_str(value)?),
            FieldType::Int => FieldValue::Int(from_b64_str(value)?),
            FieldType::Int32 => FieldValue::Int32(from_b64_str(value)?),
            FieldType::Float => FieldValue::Float(from_b64_str(value)?),
            FieldType::Decimal => FieldValue::Decimal(from_b64_str(value)?),
            FieldType::String => FieldValue::String(from_b64_str(value)?),
            FieldType::Date => FieldValue::Date(from_b64_str(value)?),
            FieldType::DateTime => FieldValue::DateTime(from_b64_str(value)?),
            FieldType::Enum(variants) => {
                let value: String = from_b64_str(value)?;

                variants.iter().find(|&v| v == &value).ok_or_else(|| {
                    Error::bad_request("invalid cursor value: not a recognized enum variant")
                })?;

                FieldValue::Enum(value.into())
            }
            FieldType::Json => {
                return Err(Error::bad_request(
                    "invalid cursor value: json value can't be used as cursor value",
                ))
            }
        }
        .into()
    } else {
        None
    };

    Ok(Cursor { id, value })
}

#[cfg(test)]
mod test {
    use crate as model;
    use crate::Cursor;
    use crate::FieldValue;
    use crate::Model;
    use chrono::DateTime;
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    #[derive(Model, Debug)]
    #[model(table_name = "example")]
    struct Example {
        #[model(id, primary_key)]
        id: Uuid,
        name: String,
        created_at: DateTime<Utc>,
    }

    #[test]
    fn test_deserialize() {
        let cursor = Cursor {
            value: FieldValue::String("John".to_string().into()).into(),
            id: Uuid::new_v4(),
        };

        let cursor_json = serde_json::to_value(&cursor).unwrap();

        let raw = serde_json::json!({
            "filter": "name = \"Jo\\\"hn\" && created_at < \"2024-02-18T00:56:50-08:00\"",
            "sort_by": "name",
            "sort_direction": "1",
            "cursor": &cursor_json,
            "limit": 25
        });

        let deserialized: Query<Example> = serde_json::from_value(raw).unwrap();

        println!("{:?}", deserialized);
    }
}
