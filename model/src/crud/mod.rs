use async_trait::async_trait;
use sqlx::{Database, FromRow, Postgres};
use uuid::Uuid;

use crate::relation::Reference;
use crate::{Error, Filter};
use crate::{Model, Query};

use self::{create::Create, delete::Delete, select::Select, update::Update};

#[async_trait]
pub trait Crud
where
    Self: Clone
        + Model
        + for<'a> FromRow<'a, <Postgres as Database>::Row>
        + Unpin
        + Sized
        + Sync
        + Send,
{
    fn select_related<'a, R>(&self, relation_name: &str) -> Result<Select<R>, Error>
    where
        R: Clone
            + Model
            + for<'b> FromRow<'b, <Postgres as Database>::Row>
            + Unpin
            + Sized
            + Sync
            + Send,
    {
        let relation = Self::relation_definitions()
            .into_iter()
            .find(|rel| rel.name == relation_name)
            .ok_or_else(|| Error::internal("undefined relation"))?;

        let filter = match relation.reference {
            Reference::From(column) => Filter::new()
                .field(&(relation.model_definition.id_field_name)())
                .eq(self.field_value(&column)?),
            Reference::To(column) => Filter::new().field(&column).eq(self.id_field_value()),
            Reference::Via((junction_table, _, _)) => {
                // find the reverse relation
                let mut related_defs =
                    (relation.model_definition.relation_definitions)().into_iter();

                let reverse_relation =
                    related_defs.find(|related_def| match &related_def.reference {
                        Reference::Via((j_table, _, _)) => j_table == &junction_table,
                        _ => false,
                    }).ok_or_else(|| Error::bad_request("reverse relation doesn't exist for many-to-many relation. reverse relation must be defined in order to retrieve related entities"))?;

                let filter_field = format!("{}.{}", reverse_relation.name, Self::id_field_name());

                Filter::new().field(&filter_field).eq(self.id_field_value())
            }
        };

        Ok(Select::new().with_filter(filter))
    }

    fn create<'a>(&'a self) -> Create<'a, Self> {
        Create::new(self)
    }

    fn update<'a>(&'a self) -> Update<'a, Self> {
        Update::new(self)
    }

    fn delete<'a>(id: &'a Uuid) -> Delete<'a, Self> {
        Delete::new(id)
    }

    fn select() -> Select<Self> {
        Select::new()
    }

    fn select_from_query(query: Query<Self>) -> Result<Select<Self>, Error> {
        query.try_into()
    }
}

impl<T> Crud for T where
    T: Clone
        + Model
        + for<'a> FromRow<'a, <Postgres as Database>::Row>
        + Unpin
        + Sized
        + Sync
        + Send
{
}

mod create;
mod delete;
mod select;
mod update;
mod util;
