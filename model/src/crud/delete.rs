use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};
use uuid::Uuid;

use crate::Error;
use crate::{crud::util::build_query, Model};

#[derive(Clone, Debug)]
pub struct Delete<'a, T: Model> {
    id: &'a Uuid,
    idempotent: bool,
    _marker: PhantomData<T>,
}

impl<'a, T> Delete<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(id: &'a Uuid) -> Self {
        Self {
            id,
            idempotent: false,
            _marker: PhantomData::default(),
        }
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    pub async fn execute(&self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let id_field_name = T::id_field_name();

        let statement = format!("DELETE FROM {} WHERE {} = $1", table_name, id_field_name);

        let var_bindings = vec![self.id.clone().into()];

        let result = build_query(&statement, var_bindings)
            .execute(executor)
            .await;

        if self.idempotent && matches!(result, Err(sqlx::Error::RowNotFound)) {
            return Ok(());
        }

        result?;

        Ok(())
    }
}
