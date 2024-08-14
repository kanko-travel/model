use sqlx::{postgres::PgRow, FromRow, PgConnection};
use uuid::Uuid;

use crate::relation::Reference;
use crate::Error;
use crate::{crud::util::build_query, FieldValue, Model};

#[derive(Clone, Debug)]
pub struct CreateAssociation<'a, T: Model> {
    value: &'a T,
    relation_name: &'a str,
    associated_id: &'a Uuid,
}

impl<'a, T> CreateAssociation<'a, T>
where
    T: Model + for<'b> FromRow<'b, PgRow> + Unpin + Sized + Send,
{
    pub(crate) fn new(value: &'a T, relation_name: &'a str, associated_id: &'a Uuid) -> Self {
        Self {
            value,
            relation_name,
            associated_id,
        }
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
                    "INSERT INTO {} ({}, {}) VALUES ($1, $2)",
                    junction_table, from_ref, to_ref
                );

                let var_bindings: Vec<FieldValue> = vec![
                    self.value.id_field_value().into(),
                    self.associated_id.clone().into(),
                ];

                build_query(&statement, var_bindings)
                    .execute(executor)
                    .await?;

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
