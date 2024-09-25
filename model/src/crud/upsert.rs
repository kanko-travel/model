use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{FieldValue, Model};

use super::util::build_query_as;

#[derive(Debug)]
pub struct Upsert<'a, T> {
    value: &'a mut T,
}

impl<'a, T> Upsert<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a mut T) -> Self {
        Self { value }
    }

    pub async fn execute(self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let fields = self.value.fields()?;

        let columns = fields
            .iter()
            .map(|(def, _)| def.name.as_ref())
            .collect::<Vec<&str>>()
            .join(", ");

        let tagged_fields = fields.iter().enumerate().map(|(i, field)| (i + 1, field));

        let placeholder_values = tagged_fields
            .clone()
            .map(|(i, _)| format!("${}", i))
            .collect::<Vec<String>>()
            .join(", ");

        let primary_key_index = format!("{}_pkey", table_name);

        let update_fields = tagged_fields.filter_map(|(i, (def, val))| {
            if def.immutable {
                None
            } else {
                (i, (def, val)).into()
            }
        });

        let placeholder_set = update_fields
            .map(|(i, (def, _))| format!("{} = ${}", def.name, i))
            .collect::<Vec<String>>()
            .join(", ");

        let statement = if placeholder_set == "" {
            format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ON CONSTRAINT {} DO NOTHING RETURNING *",
                table_name, columns, placeholder_values, primary_key_index
            )
        } else {
            format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ON CONSTRAINT {} DO UPDATE SET {} RETURNING *",
                table_name, columns, placeholder_values, primary_key_index, placeholder_set
            )
        };

        let var_bindings: Vec<FieldValue> = fields.into_iter().map(|(_, value)| value).collect();

        let upserted: T = build_query_as(&statement, var_bindings)
            .fetch_one(executor)
            .await?;

        *self.value = upserted;

        Ok(())
    }
}
