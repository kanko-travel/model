use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{crud::util::build_query, Model};

#[derive(Clone, Debug)]
pub struct Update<'a, T: Model> {
    value: &'a T,
}

impl<'a, T> Update<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a T) -> Self {
        Self { value }
    }

    pub async fn execute(&self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let id_field_name = T::id_field_name();
        let fields = self.value.fields()?;

        let id_field_value = fields.iter().find(|(def, _)| def.name == id_field_name).map(|(_, val)| val.clone()).ok_or_else(|| Error::internal("id_field_name does not match any of the fields defined on the model. this should never happen!"))?;

        let fields = fields.into_iter().filter_map(|(def, val)| {
            if def.immutable {
                None
            } else {
                (def, val).into()
            }
        });

        let set_string = fields
            .clone()
            .enumerate()
            .map(|(i, (def, _))| format!("{} = ${}", def.name, i + 2))
            .collect::<Vec<String>>()
            .join(", ");

        let statement = format!(
            "UPDATE {} SET {} WHERE {} = $1",
            table_name, set_string, id_field_name
        );

        let mut var_bindings = vec![id_field_value];
        var_bindings.extend(fields.into_iter().map(|(_, value)| value));

        build_query(&statement, var_bindings)
            .execute(executor)
            .await?;

        Ok(())
    }
}
