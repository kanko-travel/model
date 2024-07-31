use async_trait::async_trait;
use sqlx::{Database, FromRow, Postgres};
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
