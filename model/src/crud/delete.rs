use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};
use uuid::Uuid;

use crate::Error;
use crate::{crud::util::build_query, Model};

#[derive(Clone, Debug)]
pub struct Delete<T: Model> {
    id: Uuid,
    _marker: PhantomData<T>,
}

impl<T> Delete<T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(id: Uuid) -> Self {
        Self {
            id,
            _marker: PhantomData::default(),
        }
    }

    pub async fn execute(self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let id_field_name = T::id_field_name();

        let statement = format!("DELETE FROM {} WHERE {} = $1", table_name, id_field_name);

        let var_bindings = vec![self.id.into()];

        let result = build_query(&statement, var_bindings)
            .execute(executor)
            .await;

        if matches!(result, Err(sqlx::Error::RowNotFound)) {
            return Ok(());
        }

        result?;

        Ok(())
    }
}
