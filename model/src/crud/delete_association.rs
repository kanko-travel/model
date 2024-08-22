use sqlx::{postgres::PgRow, FromRow, PgConnection};
use uuid::Uuid;

use crate::relation::Reference;
use crate::Error;
use crate::{crud::util::build_query, FieldValue, Model};

#[derive(Clone, Debug)]
pub struct DeleteAssociation<'a, T: Model> {
    value: &'a T,
    relation_name: &'a str,
    associated_id: &'a Uuid,
    idempotent: bool,
}

impl<'a, T> DeleteAssociation<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a T, relation_name: &'a str, associated_id: &'a Uuid) -> Self {
        Self {
            value,
            relation_name,
            associated_id,
            idempotent: false,
        }
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    pub async fn execute(&self, executor: &mut PgConnection) -> Result<(), Error> {
        let relation_defs = T::relation_definitions();

        let relation = relation_defs
            .into_iter()
            .find(|rel| rel.name == self.relation_name)
            .ok_or_else(|| {
                Error::bad_request(
                    "invalid relation: can't create association for non-existent relation",
                )
            })?;

        match relation.reference {
            Reference::Via((junction_table, from_ref, to_ref)) => {
                let statement = format!(
                    "DELETE FROM {} WHERE {} = $1 AND {} = $2",
                    junction_table, from_ref, to_ref
                );

                let var_bindings: Vec<FieldValue> = vec![
                    self.value.id_field_value().into(),
                    self.associated_id.clone().into(),
                ];

                let result = build_query(&statement, var_bindings)
                    .execute(executor)
                    .await;

                if self.idempotent && matches!(result, Err(sqlx::Error::RowNotFound)) {
                    return Ok(());
                }

                result?;

                Ok(())
            }
            _ => {
                return Err(Error::bad_request(
                    "create_association can only be used with many-to-many relations",
                ))
            }
        }
    }
}
