use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{crud::util::build_query, FieldValue, Model};

#[derive(Clone, Debug)]
pub struct Create<'a, T: Model> {
    value: &'a T,
}

impl<'a, T> Create<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a T) -> Self {
        Self { value }
    }

    pub async fn execute(&self, executor: &mut PgConnection) -> Result<(), Error> {
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

        let statement = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name, columns, placeholder_values
        );

        let var_bindings: Vec<FieldValue> = fields.into_iter().map(|(_, value)| value).collect();

        build_query(&statement, var_bindings)
            .execute(executor)
            .await?;

        Ok(())
    }
}
