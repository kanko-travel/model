use std::marker::PhantomData;

use sqlx::PgConnection;
use uuid::Uuid;

use crate::relation::Reference;
use crate::Error;
use crate::Model;

pub struct BulkCreateAssociation<'a, T: Model> {
    relation_name: &'a str,
    iterator: Box<dyn Iterator<Item = (Uuid, Uuid)> + 'a>,
    _marker: PhantomData<T>,
}

impl<'a, T: Model> BulkCreateAssociation<'a, T> {
    pub(crate) fn new<I>(relation_name: &'a str, iter: I) -> Self
    where
        I: Iterator<Item = (Uuid, Uuid)> + 'a,
    {
        Self {
            relation_name,
            iterator: Box::new(iter),
            _marker: PhantomData::default(),
        }
    }

    pub async fn execute(self, executor: &mut PgConnection) -> Result<(), Error> {
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
                    "COPY {} ({}, {}) FROM stdin WITH (FORMAT csv, HEADER false)",
                    junction_table, from_ref, to_ref
                );

                let mut writer = executor.copy_in_raw(&statement).await?;

                for (from_id, to_id) in self.iterator {
                    let row = format!("{},{}\n", from_id.to_string(), to_id.to_string());
                    writer.send(row.as_bytes()).await?;
                }

                writer.finish().await?;

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
