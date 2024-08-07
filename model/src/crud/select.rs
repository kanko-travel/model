use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::Error;
use crate::{
    filter::ast::Expr, Connection, Cursor, FieldValue, Filter, Model, PageInfo, Query,
    SortDirection,
};

use super::{join::Join, util::build_query_as};

const DEFAULT_LIMIT: i64 = 100;

#[derive(Clone, Debug)]
pub struct Select<T: Model> {
    filters: Vec<Filter>,
    order_by: OrderBy,
    cursor: Option<Cursor>,
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

impl OrderBy {
    fn inverse(&self) -> Self {
        match self {
            OrderBy::IdAsc => Self::IdDesc,
            OrderBy::IdDesc => Self::IdAsc,
            OrderBy::SecondaryAsc(field) => Self::SecondaryDesc(field.clone()),
            OrderBy::SecondaryDesc(field) => Self::SecondaryAsc(field.clone()),
        }
    }

    fn to_string(&self, id_field_name: &str, for_join: bool) -> String {
        match &self {
            OrderBy::IdAsc => format!("{} ASC", id_field_name),
            OrderBy::IdDesc => format!("{} DESC", id_field_name),
            OrderBy::SecondaryAsc(field_name) => {
                let field_name = if for_join {
                    format!("a.{}", field_name)
                } else {
                    field_name.into()
                };

                format!("{} ASC, {} ASC", field_name, id_field_name)
            }
            OrderBy::SecondaryDesc(field_name) => {
                let field_name = if for_join {
                    format!("a.{}", field_name)
                } else {
                    field_name.into()
                };

                format!("{} DESC, {} DESC", field_name, id_field_name)
            }
        }
    }

    fn is_ascending(&self) -> bool {
        match self {
            OrderBy::IdAsc | OrderBy::SecondaryAsc(_) => true,
            _ => false,
        }
    }
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
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None)?;

        let nodes = build_query_as::<T>(&statement, var_bindings)
            .fetch_all(executor)
            .await?;

        self.paginate(nodes)
    }

    pub async fn fetch_one(&self, executor: &mut PgConnection) -> Result<T, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None)?;

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_one(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_optional(&self, executor: &mut PgConnection) -> Result<Option<T>, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters, None)?;

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
            cursor: None,
            limit: None,
            for_update: false,
            _marker: PhantomData::default(),
        }
    }

    pub(crate) fn paginate(&self, nodes: Vec<T>) -> Result<Connection<T>, Error> {
        let mut prev_cursor = None;

        let mut page_nodes = if let Some(cursor) = &self.cursor {
            let (prev, next) = split_nodes(nodes, cursor, &self.order_by)?;

            prev_cursor = prev
                .iter()
                .next()
                .map(|n| build_cursor(n, &self.order_by))
                .transpose()?;

            next
        } else {
            nodes
        };

        match self.limit {
            Some(limit) if (page_nodes.len() as i64) > limit => {
                let cursor_node = page_nodes.pop().ok_or_else(|| {
                    Error::internal("cursor_node should not be empty. this is a bug")
                })?;

                let next_cursor = build_cursor(&cursor_node, &self.order_by)?;

                Ok(Connection {
                    nodes: page_nodes,
                    page_info: PageInfo {
                        prev_cursor,
                        next_cursor: next_cursor.into(),
                    },
                })
            }
            _ => Ok(Connection {
                nodes: page_nodes,
                page_info: PageInfo {
                    prev_cursor,
                    next_cursor: None,
                },
            }),
        }
    }

    /// prepares a query statement that fetches a max size of limit * 2 + 1.
    /// includes limit + 1 rows after the provided cursor and limit rows before
    pub(crate) fn prepare(
        &self,
        exprs: Vec<Expr>,
        joined_select_clause: Option<String>,
    ) -> Result<(String, Vec<FieldValue>), Error> {
        let table_name = T::table_name();

        let id_field_name = T::id_field_name();
        let id_field_name = if joined_select_clause.is_some() {
            format!("a.{}", id_field_name)
        } else {
            id_field_name
        };

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

        let mut statement = match &self.cursor {
            Some(cursor) => {
                let mut inverse_predicates = predicates.clone();

                let cursor_filter = build_cursor_filter::<T>(
                    cursor,
                    &id_field_name,
                    &self.order_by,
                    joined_select_clause.is_some(),
                )?;

                let inverse_cursor_filter = build_cursor_filter::<T>(
                    cursor,
                    &id_field_name,
                    &self.order_by.inverse(),
                    joined_select_clause.is_some(),
                )?;

                let (sql, bindings) = cursor_filter.to_sql(var_bindings.len());
                predicates.push(sql);
                var_bindings.extend(bindings);

                let (sql, bindings) = inverse_cursor_filter.to_sql(var_bindings.len());
                inverse_predicates.push(sql);
                var_bindings.extend(bindings);

                let predicate = predicates.join(" AND ");
                let where_clause = format!("WHERE {}", predicate);

                let inverse_predicate = inverse_predicates.join(" AND ");
                let inverse_where_clause = format!("WHERE {}", inverse_predicate);

                let group_by_clause = if joined_select_clause.is_some() {
                    format!("GROUP BY {}", id_field_name)
                } else {
                    "".into()
                };

                let order_by = self
                    .order_by
                    .to_string(&id_field_name, joined_select_clause.is_some());

                let order_by_clause = format!("ORDER BY {}", order_by);

                let inverse_order_by = self
                    .order_by
                    .inverse()
                    .to_string(&id_field_name, joined_select_clause.is_some());

                let inverse_order_by_clause = format!("ORDER BY {}", inverse_order_by);

                let limit = match self.limit {
                    Some(limit) if limit > 0 => limit,
                    _ => DEFAULT_LIMIT,
                };

                let limit_clause = format!("LIMIT {}", limit + 1);

                let inverse_limit_clause = format!("LIMIT {}", limit + 1);

                let next_page_query = format!(
                    "
                            {}
                            {}
                            {}
                            {}
                            {}
                    ",
                    select_clause, where_clause, group_by_clause, order_by_clause, limit_clause
                );

                let previous_page_query = format!(
                    "
                            {}
                            {}
                            {}
                            {}
                            {}
                    ",
                    select_clause,
                    inverse_where_clause,
                    group_by_clause,
                    inverse_order_by_clause,
                    inverse_limit_clause
                );

                format!(
                    "
                        WITH aggregated AS (
                            (
                                {}
                            )

                            UNION
                            
                            (
                                {}
                            )
                        )
                        SELECT *
                        FROM aggregated
                        {}
                    ",
                    previous_page_query, next_page_query, order_by_clause
                )
            }
            _ => {
                let predicate = predicates.join(" AND ");
                let where_clause = format!("WHERE {}", predicate);

                let group_by_clause = if joined_select_clause.is_some() {
                    format!("GROUP BY {}", id_field_name)
                } else {
                    "".into()
                };

                let order_by = self
                    .order_by
                    .to_string(&id_field_name, joined_select_clause.is_some());

                let order_by_clause = format!("ORDER BY {}", order_by);

                let limit = match self.limit {
                    Some(limit) if limit > 0 => limit,
                    _ => DEFAULT_LIMIT,
                };

                let limit_clause = format!("LIMIT {}", limit + 1);

                format!(
                    "
                        {}
                        {}
                        {}
                        {}
                        {}
                    ",
                    select_clause, where_clause, group_by_clause, order_by_clause, limit_clause
                )
            }
        };

        if self.for_update {
            statement = format!(
                "
                    {}
                    FOR UPDATE
                ",
                statement
            );
        }

        // debugging
        println!("{}", statement);

        Ok((statement, var_bindings))
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

        Ok(Select {
            filters,
            order_by,
            cursor: query.cursor,
            limit: query.limit,
            for_update: false,
            _marker: PhantomData::default(),
        })
    }
}

fn build_cursor_filter<T: Model>(
    cursor: &Cursor,
    id_field_name: &str,
    order_by: &OrderBy,
    for_join: bool,
) -> Result<Expr, Error> {
    let filter = match order_by {
        OrderBy::IdAsc => Filter::new().field(id_field_name).gte(cursor.id),
        OrderBy::IdDesc => Filter::new().field(id_field_name).lte(cursor.id),
        OrderBy::SecondaryAsc(secondary) => {
            let secondary = if for_join {
                format!("a.{}", secondary)
            } else {
                secondary.into()
            };

            let secondary_value = cursor.value.clone().ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
            Filter::new()
                .field(&secondary)
                .gt(secondary_value.clone())
                .or()
                .group(
                    Filter::new()
                        .field(&secondary)
                        .gte(secondary_value)
                        .and()
                        .field(id_field_name)
                        .gte(cursor.id),
                )
        }
        OrderBy::SecondaryDesc(secondary) => {
            let secondary = if for_join {
                format!("a.{}", secondary)
            } else {
                secondary.into()
            };

            let secondary_value = cursor.value.clone().ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
            Filter::new()
                .field(&secondary)
                .lt(secondary_value.clone())
                .or()
                .group(
                    Filter::new()
                        .field(&secondary)
                        .lte(secondary_value)
                        .and()
                        .field(id_field_name)
                        .lte(cursor.id),
                )
        }
    };

    filter.build::<T>()
}

fn build_cursor<T: Model>(node: &T, order_by: &OrderBy) -> Result<Cursor, Error> {
    let cursor = match order_by {
        OrderBy::IdAsc | OrderBy::IdDesc => Cursor {
            id: node.id_field_value(),
            value: None,
        }
        .into(),
        OrderBy::SecondaryAsc(field) | OrderBy::SecondaryDesc(field) => Cursor {
            id: node.id_field_value(),
            value: node.field_value(field)?.into(),
        }
        .into(),
    };

    Ok(cursor)
}

fn split_nodes<T: Model>(
    nodes: Vec<T>,
    cursor: &Cursor,
    order_by: &OrderBy,
) -> Result<(Vec<T>, Vec<T>), Error> {
    let mut prev = vec![];
    let mut next = vec![];

    for node in nodes.into_iter() {
        let c = build_cursor(&node, order_by)?;

        if order_by.is_ascending() && &c >= cursor {
            next.push(node)
        } else if !order_by.is_ascending() && &c <= cursor {
            next.push(node)
        } else {
            prev.push(node)
        }
    }

    Ok((prev, next))
}
