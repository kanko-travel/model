use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};
use uuid::Uuid;

use crate::Error;
use crate::{crud::util::build_query, Model};

#[derive(Clone, Debug)]
pub struct Delete<'a, T: Model> {
    id: &'a Uuid,
    _marker: PhantomData<T>,
}

impl<'a, T> Delete<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(id: &'a Uuid) -> Self {
        Self {
            id,
            _marker: PhantomData::default(),
        }
    }

    pub async fn execute(&self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let id_field_name = T::id_field_name();

        let statement = format!("DELETE FROM {} WHERE {} = $1", table_name, id_field_name);

        let var_bindings = vec![self.id.clone().into()];

        build_query(&statement, var_bindings)
            .execute(executor)
            .await?;

        Ok(())
    }
}
