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
            _ => unimplemented!(),
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
