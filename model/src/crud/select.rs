use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{
    filter::ast::Expr, Connection, Cursor, FieldValue, Filter, Model, PageInfo, Query,
    SortDirection,
};

use super::{join::Join, util::build_query_as};

#[derive(Clone, Debug)]
pub struct Select<T: Model> {
    filters: Vec<Filter>,
    order_by: OrderBy,
    pub(crate) limit: Option<i64>,
    for_update: bool,
    _marker: PhantomData<T>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderBy {
    IdAsc,
    IdDesc,
    SecondaryAsc(String),
    SecondaryDesc(String),
}

impl<T: Model> Select<T> {
    /// This will override the entire state of the existing Select with the parameters defined in query
    pub fn from_query(self, query: Query<T>) -> Result<Self, Error> {
        query.try_into()
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn by_id(mut self, value: impl Into<FieldValue>) -> Self {
        let id_field = T::id_field_name();

        self.filters
            .push(Filter::new().field(&id_field).eq(value.into()));
        self
    }

    pub fn by_field(mut self, name: &str, value: impl Into<FieldValue>) -> Self {
        self.filters
            .push(Filter::new().field(name).eq(value.into()));
        self
    }

    pub fn for_update(mut self) -> Self {
        self.for_update = true;
        self
    }

    pub fn inner_join<F: Model>(self, filter: &str) -> Join<T, F> {
        Join::new_inner(self, filter.into())
    }

    pub fn cross_join<F: Model>(self) -> Join<T, F> {
        Join::new_cross(self)
    }
}

impl<T> Select<T>
where
    T: Model + for<'a> FromRow<'a, PgRow> + Unpin + Sized + Send,
{
    pub async fn fetch_page(&self, executor: &mut PgConnection) -> Result<Connection<T>, Error> {
        let nodes = self.fetch_all(executor).await?;

        self.paginate(nodes)
    }

    pub async fn fetch_all(&self, executor: &mut PgConnection) -> Result<Vec<T>, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None);

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_all(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_one(&self, executor: &mut PgConnection) -> Result<T, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None);

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_one(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_optional(&self, executor: &mut PgConnection) -> Result<Option<T>, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None);

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_optional(executor)
            .await?;

        Ok(result)
    }
}

impl<T: Model> Select<T> {
    pub(crate) fn new() -> Self {
        Self {
            filters: vec![],
            order_by: OrderBy::IdAsc,
            limit: None,
            for_update: false,
            _marker: PhantomData::default(),
        }
    }

    pub(crate) fn paginate(&self, mut nodes: Vec<T>) -> Result<Connection<T>, Error> {
        match self.limit {
            Some(limit) if (nodes.len() as i64) > limit => {
                let cursor_node = nodes.pop().ok_or_else(|| {
                    Error::internal("cursor_node should not be empty. this is a bug")
                })?;

                let next_cursor = match &self.order_by {
                    OrderBy::IdAsc | OrderBy::IdDesc => Cursor {
                        id: cursor_node.id_field_value(),
                        value: None,
                    }
                    .into(),
                    OrderBy::SecondaryAsc(field) | OrderBy::SecondaryDesc(field) => Cursor {
                        id: cursor_node.id_field_value(),
                        value: cursor_node.field_value(field)?.into(),
                    }
                    .into(),
                };

                Ok(Connection {
                    nodes,
                    page_info: PageInfo { next_cursor },
                })
            }
            _ => Ok(Connection {
                nodes,
                page_info: PageInfo { next_cursor: None },
            }),
        }
    }

    pub(crate) fn prepare(
        &self,
        exprs: Vec<Expr>,
        joined_select_clause: Option<String>,
    ) -> (String, Vec<FieldValue>) {
        let table_name = T::table_name();

        let select_clause = if let Some(s) = &joined_select_clause {
            s.into()
        } else {
            format!("SELECT * FROM {}", table_name)
        };

        let mut predicates = vec![];
        let mut var_bindings = vec![];

        for expr in exprs.into_iter() {
            let (sql, bindings) = expr.to_sql(var_bindings.len());

            predicates.push(sql);
            var_bindings.extend(bindings);
        }

        let predicate = predicates.join(" AND ");
        let where_clause = format!("WHERE {}", predicate);

        let id_field_name = T::id_field_name();
        let id_field_name = if joined_select_clause.is_some() {
            format!("a.{}", id_field_name)
        } else {
            id_field_name
        };

        let group_by_clause = if joined_select_clause.is_some() {
            format!("GROUP BY {}", id_field_name)
        } else {
            "".into()
        };

        let order_by = match &self.order_by {
            OrderBy::IdAsc => format!("{} ASC", id_field_name),
            OrderBy::IdDesc => format!("{} DESC", id_field_name),
            OrderBy::SecondaryAsc(field_name) => {
                let field_name = if joined_select_clause.is_some() {
                    format!("a.{}", field_name)
                } else {
                    field_name.into()
                };

                format!("{} ASC, {} ASC", field_name, id_field_name)
            }
            OrderBy::SecondaryDesc(field_name) => {
                let field_name = if joined_select_clause.is_some() {
                    format!("a.{}", field_name)
                } else {
                    field_name.into()
                };

                format!("{} DESC, {} DESC", field_name, id_field_name)
            }
        };

        let order_by_clause = format!("ORDER BY {}", order_by);

        let limit_clause = if let Some(limit) = self.limit {
            format!("LIMIT {}", limit + 1)
        } else {
            "".into()
        };

        let mut statement = format!(
            "
                    {}
                    {}
                    {}
                    {}
                    {}
            ",
            select_clause, where_clause, group_by_clause, order_by_clause, limit_clause
        );

        if self.for_update {
            statement = format!("{} FOR UPDATE", statement);
        }

        // debugging
        println!("{}", statement);

        (statement, var_bindings)
    }

    pub(crate) fn build_filters(&self) -> Result<Vec<Expr>, Error> {
        let mut results = vec![];

        for filter in self.filters.clone().into_iter() {
            results.push(filter.build::<T>()?);
        }

        Ok(results)
    }

    pub(crate) fn build_filters_with_foreign<J: Model>(&self) -> Result<Vec<Expr>, Error> {
        let mut results = vec![];

        for filter in self.filters.clone().into_iter() {
            results.push(filter.build_with_foreign::<T, J>()?);
        }

        Ok(results)
    }
}

impl<T: Model> TryFrom<Query<T>> for Select<T> {
    type Error = Error;
    fn try_from(query: Query<T>) -> Result<Self, Error> {
        let id_field_name = T::id_field_name();

        let order_by = match query.sort {
            Some(sort) if sort.field == id_field_name => match sort.direction {
                SortDirection::Ascending => OrderBy::IdAsc,
                SortDirection::Descending => OrderBy::IdDesc,
            },
            Some(sort) => match sort.direction {
                SortDirection::Ascending => OrderBy::SecondaryAsc(sort.field.clone()),
                SortDirection::Descending => OrderBy::SecondaryDesc(sort.field.clone()),
            },
            None => OrderBy::IdAsc,
        };

        let mut filters = vec![];
        if let Some(filter) = query.filter {
            filters.push(filter.try_into()?);
        }

        if let Some(cursor) = query.cursor {
            let cursor_filter = match &order_by {
                OrderBy::IdAsc => Filter::new().field(&id_field_name).gte(cursor.id),
                OrderBy::IdDesc => Filter::new().field(&id_field_name).lte(cursor.id),
                OrderBy::SecondaryAsc(secondary) => {
                    let secondary_value = cursor.value.ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
                    Filter::new()
                        .field(secondary)
                        .gt(secondary_value.clone())
                        .or()
                        .group(
                            Filter::new()
                                .field(secondary)
                                .gte(secondary_value)
                                .and()
                                .field(&id_field_name)
                                .gte(cursor.id),
                        )
                }
                OrderBy::SecondaryDesc(secondary) => {
                    let secondary_value = cursor.value.ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
                    Filter::new()
                        .field(secondary)
                        .lt(secondary_value.clone())
                        .or()
                        .group(
                            Filter::new()
                                .field(secondary)
                                .lte(secondary_value)
                                .and()
                                .field(&id_field_name)
                                .lte(cursor.id),
                        )
                }
            };

            filters.push(cursor_filter);
        }

        Ok(Select {
            filters,
            order_by,
            limit: query.limit.clone(),
            for_update: false,
            _marker: PhantomData::default(),
        })
    }
}
