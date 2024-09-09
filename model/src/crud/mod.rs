mod create;
mod create_association;
mod delete;
mod delete_association;
mod select;
mod update;
mod upsert;
mod util;

use async_trait::async_trait;
use create_association::CreateAssociation;
use delete_association::DeleteAssociation;
use sqlx::{Database, FromRow, Postgres};
use upsert::Upsert;
use uuid::Uuid;

use crate::{Error, Input};
use crate::{Model, Query};

use self::{create::Create, delete::Delete, select::Select, update::Update};

#[async_trait]
pub trait Crud
where
    Self: Clone
        + Model
        + Input
        + for<'a> FromRow<'a, <Postgres as Database>::Row>
        + Unpin
        + Sized
        + Sync
        + Send,
{
    fn select() -> Select<Self> {
        Select::new()
    }

    fn select_from_query(query: Query<Self>) -> Result<Select<Self>, Error> {
        query.try_into()
    }

    fn create(input: Self::InputType) -> Create<Self> {
        Create::new(input)
    }

    fn upsert(self) -> Upsert<Self> {
        Upsert::new(self)
    }

    fn update<'a>(&'a self) -> Update<'a, Self> {
        Update::new(self)
    }

    fn delete<'a>(id: &'a Uuid) -> Delete<'a, Self> {
        Delete::new(id)
    }

    fn create_association<'a>(
        &'a self,
        relation_name: &'a str,
        associated_id: &'a Uuid,
    ) -> CreateAssociation<'a, Self> {
        CreateAssociation::new(self, relation_name, associated_id)
    }

    fn delete_association<'a>(
        &'a self,
        relation_name: &'a str,
        associated_id: &'a Uuid,
    ) -> DeleteAssociation<'a, Self> {
        DeleteAssociation::new(self, relation_name, associated_id)
    }
}

impl<T> Crud for T where
    T: Clone
        + Model
        + Input
        + for<'a> FromRow<'a, <Postgres as Database>::Row>
        + Unpin
        + Sized
        + Sync
        + Send
{
}
