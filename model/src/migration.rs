use sqlx::{PgConnection, Postgres, Transaction};

use crate::Error;
use crate::{FieldDefinition, Model};

#[async_trait::async_trait]
pub trait Migration: Model {
    async fn migrate(tx: &mut Transaction<'_, Postgres>) -> Result<(), Error> {
        let table_name = Self::table_name();
        let field_definitions = Self::field_definitions();
        let create_statement = generate_create_statement(table_name, field_definitions);

        sqlx::query(&create_statement)
            .execute(tx as &mut PgConnection)
            .await?;

        Ok(())
    }
}

fn generate_create_statement(
    table_name: String,
    field_definitions: Vec<FieldDefinition>,
) -> String {
    let columns = field_definitions
        .iter()
        .map(|def| {
            let mut col = format!("{} {}", def.name, def.type_.sql_type());

            if !def.nullable {
                col = format!("{} {}", col, "NOT NULL");
            }

            if def.unique {
                col = format!("{} {}", col, "UNIQUE");
            }

            col
        })
        .collect::<Vec<String>>()
        .join(", ");

    let primary_key = field_definitions
        .iter()
        .filter(|def| def.primary_key)
        .map(|def| def.name.as_str())
        .collect::<Vec<&str>>()
        .join(", ");

    format!(
        "CREATE TABLE IF NOT EXISTS {} ({}, PRIMARY KEY ({}))",
        table_name, columns, primary_key
    )
}

impl<T: Model> Migration for T {}
