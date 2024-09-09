use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::{crud::util::build_query, FieldValue, Model};
use crate::{Error, Input};

use super::util::build_query_as;

#[derive(Debug)]
pub struct Create<T> {
    value: T,
    idempotent: bool,
}

impl<T> Create<T>
where
    T: Model + Input + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(input: T::InputType) -> Self {
        Self {
            value: T::from_input(input),
            idempotent: false,
        }
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;

        self
    }

    pub async fn execute(self, executor: &mut PgConnection) -> Result<T, Error> {
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

            return Ok(created);
        }

        let statement = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name, columns, placeholder_values
        );

        build_query(&statement, var_bindings)
            .execute(executor)
            .await?;

        Ok(self.value)
    }
}
