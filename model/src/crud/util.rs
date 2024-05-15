use sqlx::{
    postgres::{PgArguments, PgRow},
    query::{Query as SqlxQuery, QueryAs},
    types::Json,
    FromRow, Postgres,
};

use crate::FieldValue;

pub fn build_query<'b>(
    statement: &'b str,
    var_bindings: Vec<FieldValue>,
) -> SqlxQuery<'b, Postgres, PgArguments> {
    let mut q = sqlx::query(statement);

    for value in var_bindings.into_iter() {
        q = bind_query(q, value);
    }

    q
}

pub fn build_query_as<'a, T>(
    statement: &'a str,
    var_bindings: Vec<FieldValue>,
) -> QueryAs<'a, Postgres, T, PgArguments>
where
    T: for<'b> FromRow<'b, PgRow>,
{
    let mut q = sqlx::query_as(statement);
    for value in var_bindings.into_iter() {
        q = bind_query_as(q, value);
    }

    q
}

pub fn bind_query<'a>(
    q: SqlxQuery<'a, Postgres, PgArguments>,
    value: FieldValue,
) -> SqlxQuery<'a, Postgres, PgArguments> {
    match value {
        FieldValue::Uuid(inner) => q.bind(inner),
        FieldValue::Bool(inner) => q.bind(inner),
        FieldValue::Int(inner) => q.bind(inner),
        FieldValue::Float(inner) => q.bind(inner),
        FieldValue::Decimal(inner) => q.bind(inner),
        FieldValue::String(inner) => q.bind(inner),
        FieldValue::Date(inner) => q.bind(inner),
        FieldValue::DateTime(inner) => q.bind(inner),
        FieldValue::Enum(inner) => q.bind(inner),
        FieldValue::Json(inner) => q.bind(Json(inner)),
    }
}

pub fn bind_query_as<'a, T>(
    q: QueryAs<'a, Postgres, T, PgArguments>,
    value: FieldValue,
) -> QueryAs<'a, Postgres, T, PgArguments> {
    match value {
        FieldValue::Uuid(inner) => q.bind(inner),
        FieldValue::Bool(inner) => q.bind(inner),
        FieldValue::Int(inner) => q.bind(inner),
        FieldValue::Float(inner) => q.bind(inner),
        FieldValue::Decimal(inner) => q.bind(inner),
        FieldValue::String(inner) => q.bind(inner),
        FieldValue::Date(inner) => q.bind(inner),
        FieldValue::DateTime(inner) => q.bind(inner),
        FieldValue::Enum(inner) => q.bind(inner),
        FieldValue::Json(inner) => q.bind(Json(inner)),
    }
}
