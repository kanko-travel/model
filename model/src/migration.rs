use sqlx::{PgConnection, Postgres, Transaction};

use crate::relation::{Reference, RelationDef};
use crate::Error;
use crate::{FieldDefinition, Model};

#[async_trait::async_trait]
pub trait Migration: Model {
    async fn migrate(tx: &mut Transaction<'_, Postgres>) -> Result<(), Error> {
        let table_name = Self::table_name();
        let field_definitions = Self::field_definitions();
        let relation_definitions = Self::relation_definitions();

        let create_statement =
            generate_create_statement(table_name, field_definitions, relation_definitions);

        sqlx::query(&create_statement)
            .execute(tx as &mut PgConnection)
            .await?;

        Ok(())
    }

    async fn migrate_junction_tables(tx: &mut Transaction<'_, Postgres>) -> Result<(), Error> {
        let relation_defs = Self::relation_definitions();

        for def in relation_defs.iter() {
            match &def.reference {
                Reference::Via((junction_table, from_ref, to_ref)) => {
                    let from_table = Self::table_name();
                    let from_table_id_field = Self::id_field_name();
                    let to_table = (def.model_definition.table_name)();
                    let to_table_id_field = (def.model_definition.id_field_name)();

                    let columns = format!("{} UUID NOT NULL, {} UUID NOT NULL", from_ref, to_ref);
                    let primary_key = format!("PRIMARY KEY({}, {})", from_ref, to_ref);
                    let from_foreign_key_constraint = format!(
                        "CONSTRAINT fk_from_reference FOREIGN KEY({}) REFERENCES {}({})",
                        from_ref, from_table, from_table_id_field
                    );
                    let to_foreign_key_constraint = format!(
                        "CONSTRAINT fk_to_reference FOREIGN KEY({}) REFERENCES {}({})",
                        to_ref, to_table, to_table_id_field
                    );

                    let create_statement = format!(
                        "CREATE TABLE IF NOT EXISTS {} ({}, {}, {}, {})",
                        junction_table,
                        columns,
                        primary_key,
                        from_foreign_key_constraint,
                        to_foreign_key_constraint
                    );

                    sqlx::query(&create_statement)
                        .execute(tx as &mut PgConnection)
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn generate_create_statement(
    table_name: String,
    field_definitions: Vec<FieldDefinition>,
    relation_definitions: Vec<RelationDef>,
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

    let foreign_key_constraints = relation_definitions
        .iter()
        .filter_map(|rel| match &rel.reference {
            Reference::From(column) => {
                let foreign_table = (rel.model_definition.table_name)();
                let foreign_column = (rel.model_definition.id_field_name)();

                format!(
                    "CONSTRAINT fk_{} FOREIGN KEY({}) REFERENCES {}({})",
                    rel.name, column, foreign_table, foreign_column
                )
                .into()
            }
            _ => None,
        })
        .collect::<Vec<String>>()
        .join(",");

    let statement = if foreign_key_constraints == "" {
        format!(
            "CREATE TABLE IF NOT EXISTS {} ({}, PRIMARY KEY ({}))",
            table_name, columns, primary_key
        )
    } else {
        format!(
            "CREATE TABLE IF NOT EXISTS {} ({}, PRIMARY KEY({}), {})",
            table_name, columns, primary_key, foreign_key_constraints
        )
    };

    statement
}

impl<T: Model> Migration for T {}
