use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{crud::util::build_query, FieldValue, Model};

use super::util::build_query_as;

#[derive(Debug)]
pub struct Create<'a, T: Model> {
    value: &'a mut T,
    idempotent: bool,
}

impl<'a, T> Create<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a mut T) -> Self {
        Self {
            value,
            idempotent: false,
        }
    }

    pub async fn execute(&mut self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let fields = self.value.fields()?;

        let columns = fields
            .iter()
            .map(|(def, _)| def.name.as_ref())
            .collect::<Vec<&str>>()
            .join(", ");

        let placeholder_values = fields
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect::<Vec<String>>()
            .join(", ");

        let var_bindings: Vec<FieldValue> = fields.into_iter().map(|(_, value)| value).collect();

        if self.idempotent {
            let primary_key_index = format!("{}_pkey", table_name);

            let statement = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ON CONSTRAINT {} DO NOTHING RETURNING *",
                table_name, columns, placeholder_values, primary_key_index
            );

            let created: T = build_query_as(&statement, var_bindings)
                .fetch_one(executor)
                .await?;

            *self.value = created;

            return Ok(());
        }

        let statement = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name, columns, placeholder_values
        );

        build_query(&statement, var_bindings)
            .execute(executor)
            .await?;

        Ok(())
    }
}
