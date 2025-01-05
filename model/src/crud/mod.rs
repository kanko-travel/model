mod bulk_create;
mod bulk_create_association;
mod create;
mod create_association;
mod delete;
mod delete_association;
mod select;
mod update;
mod upsert;
mod util;

use async_trait::async_trait;
use bulk_create::BulkCreate;
use bulk_create_association::BulkCreateAssociation;
use create_association::CreateAssociation;
use delete_association::DeleteAssociation;
use sqlx::{Database, FromRow, Postgres};
use upsert::Upsert;
use uuid::Uuid;

use crate::Error;
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
    fn create<'a>(&'a self) -> Create<'a, Self> {
        Create::new(self)
    }

    fn upsert<'a>(&'a mut self) -> Upsert<'a, Self> {
        Upsert::new(self)
    }

    fn update<'a>(&'a self) -> Update<'a, Self> {
        Update::new(self)
    }

    fn delete<'a>(&'a self) -> Delete<Self> {
        Delete::new(self.id_field_value())
    }

    fn delete_by_id<Id: Into<Uuid>>(id: Id) -> Delete<Self> {
        Delete::new(id.into())
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

    fn bulk_create<'a, I>(iter: I) -> BulkCreate<'a, Self>
    where
        I: Iterator<Item = Self> + 'a,
    {
        BulkCreate::new(iter)
    }

    fn bulk_create_association<'a, I>(
        relation_name: &'a str,
        iter: I,
    ) -> BulkCreateAssociation<'a, Self>
    where
        I: Iterator<Item = (Uuid, Uuid)> + 'a,
    {
        BulkCreateAssociation::new(relation_name, iter)
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
