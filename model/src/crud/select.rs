use std::collections::HashSet;
use std::marker::PhantomData;

use sqlx::{postgres::PgRow, FromRow, PgConnection};

use crate::filter::ast::Var;
use crate::{
    filter::ast::Expr, Connection, Cursor, FieldValue, Filter, Model, PageInfo, Query,
    SortDirection,
};
use crate::{Error, ModelDef};

use super::util::build_query_as;

const DEFAULT_LIMIT: i64 = 100;

#[derive(FromRow)]
pub struct WithCursor<T> {
    #[sqlx(flatten)]
    node: T,
    _cursor: Option<String>,
    _next_page: bool,
}

#[derive(Clone, Debug)]
pub struct Select<T: Model> {
    select_path: String,
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
    SecondaryAsc(Var),
    SecondaryDesc(Var),
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

    fn selects<T: Model>(&self) -> String {
        let primary_field_reference = self.primary_field_reference::<T>();
        match &self {
            OrderBy::IdAsc | OrderBy::IdDesc => {
                format!(
                    "{}::text AS _cursor, {} AS _order_by_primary",
                    primary_field_reference, primary_field_reference
                )
            }
            OrderBy::SecondaryAsc(var) | OrderBy::SecondaryDesc(var) => {
                if matches!(var, Var::Node(_)) {
                    format!(
                        "MAX({})::text AS _cursor, MAX({}) AS _order_by_primary, {}.{} AS _order_by_secondary",
                        primary_field_reference,
                        primary_field_reference,
                        T::table_name(),
                        T::id_field_name()
                    )
                } else {
                    format!(
                        "{}::text AS _cursor, {} AS _order_by_primary, {}.{} AS _order_by_secondary",
                        primary_field_reference,
                        primary_field_reference,
                        T::table_name(),
                        T::id_field_name()
                    )
                }
            }
        }
    }

    fn to_sql<T: Model>(&self) -> String {
        match &self {
            OrderBy::IdAsc => "_order_by_primary ASC".into(),
            OrderBy::SecondaryAsc(_) => "_order_by_primary ASC, _order_by_secondary ASC".into(),
            OrderBy::IdDesc => "_order_by_primary DESC".into(),
            OrderBy::SecondaryDesc(_) => "_order_by_primary DESC, _order_by_secondary DESC".into(),
        }
    }

    fn primary_field_reference<T: Model>(&self) -> String {
        match &self {
            OrderBy::IdAsc | OrderBy::IdDesc => {
                format!("{}.{}", T::table_name(), T::id_field_name())
            }
            OrderBy::SecondaryAsc(var) | OrderBy::SecondaryDesc(var) => {
                let mut reference = var.to_sql();

                if matches!(var, Var::Leaf(_)) {
                    reference = format!("{}.{}", T::table_name(), reference);
                }

                reference
            }
        }
    }

    fn references_relation(&self) -> bool {
        if matches!(self, OrderBy::SecondaryAsc(Var::Node(_)))
            || matches!(self, OrderBy::SecondaryDesc(Var::Node(_)))
        {
            true
        } else {
            false
        }
    }
}

impl<T: Model> Select<T> {
    /// This will override the entire state of the existing Select with the parameters defined in query
    pub fn from_query(self, query: Query<T>) -> Result<Self, Error> {
        query.try_into()
    }

    pub fn with_query(mut self, query: Query<T>) -> Result<Self, Error> {
        let other: Select<T> = query.try_into()?;

        self.filters.extend(other.filters);
        self.limit = other.limit;
        self.order_by = other.order_by;
        self.cursor = other.cursor;

        Ok(self)
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
}

impl<T> Select<T>
where
    T: Model + for<'a> FromRow<'a, PgRow> + Unpin + Sized + Send,
{
    pub async fn fetch_all(mut self, executor: &mut PgConnection) -> Result<Vec<T>, Error> {
        self.limit = None;
        self.cursor = None;

        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters)?;

        let nodes = build_query_as::<WithCursor<T>>(&statement, var_bindings)
            .fetch_all(executor)
            .await?;

        Ok(nodes.into_iter().map(|node| node.node).collect())
    }

    pub async fn fetch_page(mut self, executor: &mut PgConnection) -> Result<Connection<T>, Error> {
        self.limit = match self.limit {
            Some(limit) if limit > 0 => limit.into(),
            _ => DEFAULT_LIMIT.into(),
        };

        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters)?;

        let nodes = build_query_as::<WithCursor<T>>(&statement, var_bindings)
            .fetch_all(executor)
            .await?;

        self.paginate(nodes)
    }

    pub async fn fetch_one(self, executor: &mut PgConnection) -> Result<T, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters)?;

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_one(executor)
            .await?;

        Ok(result)
    }

    pub async fn fetch_optional(self, executor: &mut PgConnection) -> Result<Option<T>, Error> {
        let filters = self.build_filters()?;
        let (statement, var_bindings) = self.prepare(filters)?;

        let result = build_query_as::<T>(&statement, var_bindings)
            .fetch_optional(executor)
            .await?;

        Ok(result)
    }
}

impl<T: Model> Select<T> {
    pub(crate) fn new() -> Self {
        Self {
            select_path: T::table_name(),
            filters: vec![],
            order_by: OrderBy::IdAsc,
            cursor: None,
            limit: None,
            for_update: false,
            _marker: PhantomData::default(),
        }
    }

    pub(crate) fn paginate(&self, nodes: Vec<WithCursor<T>>) -> Result<Connection<T>, Error> {
        let mut prev_cursor = None;

        let mut page_nodes = if let Some(cursor) = &self.cursor {
            tracing::info!("BEGIN RETURNED ROWS, COUNT = {}", nodes.len());
            nodes.iter().for_each(|node| {
                tracing::debug!("{:?}\n", node._cursor);
            });
            tracing::info!("END RETURNED ROWS");

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
                    nodes: page_nodes.into_iter().map(|n| n.node).collect(),
                    page_info: PageInfo {
                        prev_cursor,
                        next_cursor: next_cursor.into(),
                    },
                })
            }
            _ => Ok(Connection {
                nodes: page_nodes.into_iter().map(|n| n.node).collect(),
                page_info: PageInfo {
                    prev_cursor,
                    next_cursor: None,
                },
            }),
        }
    }

    /// prepares a query statement that fetches a max size of limit * 2 + 1.
    /// includes limit + 1 rows after the provided cursor and limit rows before
    pub(crate) fn prepare(&self, exprs: Vec<Expr>) -> Result<(String, Vec<FieldValue>), Error> {
        let table_name = T::table_name();
        let id_field_name = T::id_field_name();

        let select_clause = format!(
            "SELECT {}.*, {}, TRUE AS _next_page FROM {}",
            self.select_path,
            self.order_by.selects::<T>(),
            table_name
        );

        let order_by_references_relation = self.order_by.references_relation();

        let mut vars = vec![];

        match &self.order_by {
            OrderBy::SecondaryAsc(var) | OrderBy::SecondaryDesc(var) => {
                vars.push(var.clone());
            }
            _ => {}
        }

        let mut predicates = vec![];
        let mut var_bindings = vec![];

        for expr in exprs.into_iter() {
            let (sql, v, b) = expr.to_sql::<T>(var_bindings.len());

            tracing::info!("pushing predicate: {}", sql);

            predicates.push(sql);
            vars.extend(v);
            var_bindings.extend(b);
        }

        let mut statement = match &self.cursor {
            Some(cursor) => {
                let inverse_select_clause = format!(
                    "SELECT {}.*, {}, FALSE AS _next_page FROM {}",
                    self.select_path,
                    self.order_by.selects::<T>(),
                    table_name
                );

                let mut inverse_predicates = predicates.clone();

                tracing::info!("building cursor filter");

                let cursor_filter =
                    build_cursor_filter::<T>(cursor, &id_field_name, &self.order_by)?;

                let inverse_cursor_filter =
                    build_cursor_filter::<T>(cursor, &id_field_name, &self.order_by.inverse())?;

                let (sql, v, b) = cursor_filter.to_sql::<T>(var_bindings.len());

                tracing::info!("cursor predicate: {}", sql);

                predicates.push(sql);
                vars.extend(v);
                var_bindings.extend(b);

                let (sql, _, b) = inverse_cursor_filter.to_sql::<T>(var_bindings.len());

                tracing::info!("inverse cursor predicate: {}", sql);

                inverse_predicates.push(sql);
                var_bindings.extend(b);

                tracing::info!("building join clause");

                let join_clause = generate_join_clause::<T>(&vars)?;

                tracing::info!("join clause: {}", join_clause);

                let predicate = predicates.join(" AND ");
                let where_clause = format!("WHERE {}", predicate);

                let inverse_predicate = inverse_predicates.join(" AND ");
                let inverse_where_clause = format!("WHERE {}", inverse_predicate);

                let group_by_clause =
                    generate_group_by_clause::<T>(join_clause != "", order_by_references_relation);

                tracing::info!("building order_by clause");

                let order_by = self.order_by.to_sql::<T>();
                let order_by_clause = format!("ORDER BY {}", order_by);

                tracing::info!("order_by clause: {}", order_by_clause);

                tracing::info!("building inverse_order_by cluase");

                let inverse_order_by = self.order_by.inverse().to_sql::<T>();
                let inverse_order_by_clause = format!("ORDER BY {}", inverse_order_by);

                tracing::info!("inverse_order_by clause: {}", inverse_order_by_clause);

                let limit_clause = match self.limit {
                    Some(limit) if limit > 0 => format!("LIMIT {}", limit + 1),
                    _ => "".into(),
                };

                let next_page_query = format!(
                    "
                            {}
                            {}
                            {}
                            {}
                            {}
                            {}
                    ",
                    select_clause,
                    join_clause,
                    where_clause,
                    group_by_clause,
                    order_by_clause,
                    limit_clause
                );

                let previous_page_query = format!(
                    "
                            {}
                            {}
                            {}
                            {}
                            {}
                            {}
                    ",
                    inverse_select_clause,
                    join_clause,
                    inverse_where_clause,
                    group_by_clause,
                    inverse_order_by_clause,
                    limit_clause
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
                tracing::info!("building join clause");

                let join_clause = generate_join_clause::<T>(&vars)?;

                tracing::info!("join clause: {}", join_clause);

                let predicate = predicates.join(" AND ");
                let where_clause = if predicate != "" {
                    format!("WHERE {}", predicate)
                } else {
                    "".into()
                };

                let group_by_clause =
                    generate_group_by_clause::<T>(join_clause != "", order_by_references_relation);

                tracing::info!("building order_by clause");

                let order_by = self.order_by.to_sql::<T>();
                let order_by_clause = format!("ORDER BY {}", order_by);

                tracing::info!("order_by clause: {}", order_by_clause);

                let limit_clause = match self.limit {
                    Some(limit) if limit > 0 => format!("LIMIT {}", limit + 1),
                    _ => "".into(),
                };

                format!(
                    "
                        {}
                        {}
                        {}
                        {}
                        {}
                        {}
                    ",
                    select_clause,
                    join_clause,
                    where_clause,
                    group_by_clause,
                    order_by_clause,
                    limit_clause
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
        tracing::info!("{}", statement);

        Ok((statement, var_bindings))
    }

    pub(crate) fn build_filters(&self) -> Result<Vec<Expr>, Error> {
        let mut results = vec![];

        for filter in self.filters.clone().into_iter() {
            results.push(filter.build::<T>()?);
        }

        Ok(results)
    }
}

impl<T: Model> TryFrom<Query<T>> for Select<T> {
    type Error = Error;
    fn try_from(query: Query<T>) -> Result<Self, Error> {
        let id_field_name = T::id_field_name();

        let order_by = match query.sort {
            Some(sort) if sort.field.to_string() == id_field_name => match sort.direction {
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
            select_path: T::table_name(),
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
) -> Result<Expr, Error> {
    let filter = match order_by {
        OrderBy::IdAsc => Filter::new().field(id_field_name).gte(cursor.id),
        OrderBy::IdDesc => Filter::new().field(id_field_name).lte(cursor.id),
        OrderBy::SecondaryAsc(secondary) => {
            let secondary = secondary.to_string();

            let secondary_value = cursor.value.clone().ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
            Filter::new()
                .field(&secondary)
                .gt(secondary_value.clone())
                .or()
                .group(
                    Filter::new()
                        .field(&secondary)
                        .eq(secondary_value)
                        .and()
                        .field(id_field_name)
                        .gte(cursor.id),
                )
        }
        OrderBy::SecondaryDesc(secondary) => {
            let secondary = secondary.to_string();

            let secondary_value = cursor.value.clone().ok_or_else(|| Error::bad_request("invalid cursor: a cursor containing a value referencing the sort_by field is required"))?;
            Filter::new()
                .field(&secondary)
                .lt(secondary_value.clone())
                .or()
                .group(
                    Filter::new()
                        .field(&secondary)
                        .eq(secondary_value)
                        .and()
                        .field(id_field_name)
                        .lte(cursor.id),
                )
        }
    };

    filter.build::<T>()
}

fn build_cursor<T: Model>(node: &WithCursor<T>, order_by: &OrderBy) -> Result<Cursor, Error> {
    let cursor = match order_by {
        OrderBy::IdAsc | OrderBy::IdDesc => Cursor {
            id: node.node.id_field_value(),
            value: None,
        }
        .into(),
        OrderBy::SecondaryAsc(var) | OrderBy::SecondaryDesc(var) => {
            let model_def = T::definition();
            let def = var.resolve_definition(&model_def)?;

            let value = node
                ._cursor
                .as_ref()
                .map(|cursor| def.type_.parse_value(cursor))
                .transpose()?
                .or_else(|| def.type_.null_value().into());

            Cursor {
                id: node.node.id_field_value(),
                value,
            }
        }
        .into(),
    };

    Ok(cursor)
}

fn split_nodes<T: Model>(
    nodes: Vec<WithCursor<T>>,
    cursor: &Cursor,
    order_by: &OrderBy,
) -> Result<(Vec<WithCursor<T>>, Vec<WithCursor<T>>), Error> {
    let mut prev = vec![];
    let mut next = vec![];

    for node in nodes.into_iter() {
        let c = build_cursor(&node, order_by)?;

        if node._next_page {
            next.push(node);
            continue;
        }

        if &c != cursor && !node._next_page {
            prev.push(node)
        }
    }

    Ok((prev, next))
}

fn generate_join_clause<T: Model>(vars: &Vec<Var>) -> Result<String, Error> {
    let mut seen = HashSet::new();
    let mut join_clauses = vec![];

    let table_name = T::table_name();

    for var in vars.iter() {
        let joins = joins_from_var(&table_name, &table_name, var, &T::definition())?;

        for (relation, join_clause) in joins.iter() {
            if seen.insert(relation.clone()) {
                join_clauses.push(join_clause.clone());
            }
        }
    }

    Ok(join_clauses.join("\n"))
}

fn joins_from_var(
    root: &str,
    parent: &str,
    var: &Var,
    model_def: &ModelDef,
) -> Result<Vec<(String, String)>, Error> {
    match var {
        Var::Leaf(_) => Ok(vec![]),
        Var::Node((name, var)) => {
            let relation_defs = (model_def.relation_definitions)();
            let relation_def = relation_defs
                .iter()
                .find(|def| &def.name == name)
                .ok_or_else(|| Error::bad_request("undefined field"))?;

            let id_field_name = (model_def.id_field_name)();
            let join_clause = relation_def.to_join_clause(&parent, &id_field_name, root == parent);

            let next_parent = if root == parent {
                name.clone()
            } else {
                format!("{}_{}", parent, name)
            };

            let mut res = vec![(next_parent.clone(), join_clause)];

            res.extend(joins_from_var(
                &root,
                &next_parent,
                var.as_ref(),
                &relation_def.model_definition,
            )?);

            Ok(res)
        }
    }
}

fn generate_group_by_clause<T: Model>(
    references_relation: bool,
    order_by_references_relation: bool,
) -> String {
    let table_name = T::table_name();

    if !references_relation {
        return "".into();
    }

    if !order_by_references_relation {
        // generate a list of each primary_key field
        let fields_string = T::field_definitions()
            .into_iter()
            .filter_map(|field_def| {
                field_def
                    .primary_key
                    .then(|| format!("{}.{}", table_name, field_def.name))
            })
            .collect::<Vec<_>>()
            .join(", ");

        return format!("GROUP BY {}", fields_string);
    }

    // generate a list of each field
    let fields_string = T::field_definitions()
        .into_iter()
        .map(|field_def| format!("{}.{}", table_name, field_def.name))
        .collect::<Vec<_>>()
        .join(", ");

    format!("GROUP BY {}", fields_string)
}
