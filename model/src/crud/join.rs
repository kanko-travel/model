use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{Connection, FieldValue, Filter, Model};

use super::{select::Select, util::build_query_as};

#[derive(Clone, Debug)]
pub enum JoinType {
    Inner(String),
    Cross,
}

#[derive(Clone, Debug)]
pub struct Join<S: Model, F: Model> {
    join_type: JoinType,
    select: Select<S>,
    _marker: PhantomData<F>,
}

impl<S, F> Join<S, F>
where
    S: Model,
    F: Model,
{
    pub(crate) fn new_inner(select: Select<S>, filter: String) -> Self {
        Self {
            join_type: JoinType::Inner(filter),
            select,
            _marker: PhantomData::default(),
        }
    }

    pub(crate) fn new_cross(select: Select<S>) -> Self {
        Self {
            join_type: JoinType::Cross,
            select,
            _marker: PhantomData::default(),
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.select = self.select.with_filter(filter);
        self
    }

    pub fn by_id(mut self, value: impl Into<FieldValue>) -> Self {
        self.select = self.select.by_id(value);
        self
    }

    pub fn by_field(mut self, name: &str, value: impl Into<FieldValue>) -> Self {
        self.select = self.select.by_field(name, value);
        self
    }

    pub fn by_foreign_field(self, name: &str, value: impl Into<FieldValue>) -> Self {
        let filter = Filter::new().foreign_field(name).eq(value);
        self.with_filter(filter)
    }

    pub fn for_update(mut self) -> Self {
        self.select = self.select.for_update();
        self
    }
}

impl<S, F> Join<S, F>
where
    S: Model + for<'a> FromRow<'a, PgRow> + Unpin + Sized + Send,
    F: Model,
{
    pub async fn fetch_page(&self, executor: &mut PgConnection) -> Result<Connection<S>, Error> {
        let nodes = self.fetch_all(executor).await?;

        self.select.paginate(nodes)
    }

    pub async fn fetch_all(&self, executor: &mut PgConnection) -> Result<Vec<S>, Error> {
        let filters = self.select.build_filters_with_foreign::<F>()?;
        let select_clause = self.prepare_select_clause();

        let (statement, var_bindings) = self.select.prepare(filters, select_clause.into());

        let result = build_query_as::<S>(&statement, var_bindings)
            .fetch_all(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_one(&self, executor: &mut PgConnection) -> Result<S, Error> {
        let filters = self.select.build_filters_with_foreign::<F>()?;
        let select_clause = self.prepare_select_clause();

        let (statement, var_bindings) = self.select.prepare(filters, select_clause.into());

        let result = build_query_as::<S>(&statement, var_bindings)
            .fetch_one(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_optional(&self, executor: &mut PgConnection) -> Result<Option<S>, Error> {
        let filters = self.select.build_filters_with_foreign::<F>()?;
        let select_clause = self.prepare_select_clause();

        let (statement, var_bindings) = self.select.prepare(filters, select_clause.into());

        let result = build_query_as::<S>(&statement, var_bindings)
            .fetch_optional(executor)
            .await?;

        Ok(result)
    }
}

impl<S, F> Join<S, F>
where
    S: Model,
    F: Model,
{
    fn prepare_select_clause(&self) -> String {
        let table_a = S::table_name();
        let table_b = F::table_name();

        match &self.join_type {
            JoinType::Cross => format!(
                "SELECT a.* FROM {} AS a CROSS JOIN {} as b",
                table_a, table_b
            ),
            JoinType::Inner(filter) => format!(
                "SELECT a.* FROM {} AS a INNER JOIN {} as b ON {}",
                table_a, table_b, filter
            ),
        }
    }
}
