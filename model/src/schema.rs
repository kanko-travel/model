use std::collections::HashSet;

use crate::index::IndexType;
use crate::relation::Reference;
use crate::Error;
use crate::Model;

#[macro_export]
macro_rules! schema {
    ($($t:ty),*) => {
        {
            use model::DDL;
            let mut entities = Vec::new();

            $(
                entities.extend(<$t>::ddl());
            )*

            model::generate_schema(&entities).unwrap()
        }
    };
}

#[macro_export]
macro_rules! schema_parts {
    ($($t:ty),*) => {
        {
            use model::DDL;
            let mut entities = Vec::new();

            $(
                entities.extend(<$t>::ddl());
            )*

            (
                model::generate_table_schema(&entities).unwrap(),
                model::generate_fkey_schema(&entities).unwrap(),
                model::generate_index_schema(&entities).unwrap()
            )
        }
    };
}

#[macro_export]
macro_rules! schema_entities {
    ($($t:ty),*) => {
        {
            use model::DDL;
            let mut entities = Vec::new();

            $(
                entities.extend(<$t>::ddl());
            )*

            entities
        }
    };
}

pub trait DDL {
    fn ddl() -> Vec<DDLEntity>;
}

pub enum DDLEntity {
    ForeignKeyConstraint((String, String)),
    Table((String, String)),
    JunctionTable((String, String)),
    Index((String, String)),
}

impl DDLEntity {
    fn id(&self) -> String {
        match self {
            DDLEntity::ForeignKeyConstraint((id, _))
            | DDLEntity::JunctionTable((id, _))
            | DDLEntity::Table((id, _))
            | DDLEntity::Index((id, _)) => id.into(),
        }
    }
    fn ddl(&self) -> String {
        match self {
            DDLEntity::ForeignKeyConstraint((_, ddl))
            | DDLEntity::JunctionTable((_, ddl))
            | DDLEntity::Table((_, ddl))
            | DDLEntity::Index((_, ddl)) => ddl.into(),
        }
    }
}

impl<T: Model> DDL for T {
    fn ddl() -> Vec<DDLEntity> {
        let mut entities = vec![create_table::<T>()];
        entities.extend(create_junction_tables_and_foreign_keys::<T>());
        entities.extend(create_indices::<T>());

        entities
    }
}

pub fn generate_schema(entities: &Vec<DDLEntity>) -> Result<String, Error> {
    let mut ids = HashSet::new();

    let mut ddl = vec![];

    for entity in entities.iter().filter(|e| matches!(e, DDLEntity::Table(_))) {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    for entity in entities
        .iter()
        .filter(|e| matches!(e, DDLEntity::JunctionTable(_)))
    {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    for entity in entities
        .iter()
        .filter(|e| matches!(e, DDLEntity::ForeignKeyConstraint(_)))
    {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    for entity in entities.iter().filter(|e| matches!(e, DDLEntity::Index(_))) {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    let ddl = ddl.join("\n\n");

    Ok(ddl)
}

pub fn generate_table_schema(entities: &Vec<DDLEntity>) -> Result<String, Error> {
    let mut ids = HashSet::new();

    let mut ddl = vec![];

    for entity in entities.iter().filter(|e| matches!(e, DDLEntity::Table(_))) {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    for entity in entities
        .iter()
        .filter(|e| matches!(e, DDLEntity::JunctionTable(_)))
    {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    let ddl = ddl.join("\n\n");

    Ok(ddl)
}

pub fn generate_fkey_schema(entities: &Vec<DDLEntity>) -> Result<String, Error> {
    let mut ids = HashSet::new();

    let mut ddl = vec![];

    for entity in entities
        .iter()
        .filter(|e| matches!(e, DDLEntity::ForeignKeyConstraint(_)))
    {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    let ddl = ddl.join("\n\n");

    Ok(ddl)
}

pub fn generate_index_schema(entities: &Vec<DDLEntity>) -> Result<String, Error> {
    let mut ids = HashSet::new();

    let mut ddl = vec![];

    for entity in entities.iter().filter(|e| matches!(e, DDLEntity::Index(_))) {
        if ids.insert(entity.id()) {
            ddl.push(entity.ddl());
        }
    }

    let ddl = ddl.join("\n\n");

    Ok(ddl)
}

fn create_table<T: Model>() -> DDLEntity {
    let table_name = T::table_name();
    let field_definitions = T::field_definitions();

    let columns = field_definitions
        .iter()
        .map(|def| {
            let mut col = format!("{} {}", def.name, def.type_.sql_type());

            if !def.nullable {
                col = format!("{} {}", col, "NOT NULL");
            }

            if def.unique {
                col = format!("{} {}", col, "UNIQUE");
            }

            col
        })
        .collect::<Vec<String>>()
        .join(", ");

    let primary_key = field_definitions
        .iter()
        .filter(|def| def.primary_key)
        .map(|def| def.name.as_str())
        .collect::<Vec<&str>>()
        .join(", ");

    let statement = format!(
        "CREATE TABLE {} ({}, PRIMARY KEY ({}));",
        table_name, columns, primary_key
    );

    DDLEntity::Table((table_name, statement))
}

fn create_junction_tables_and_foreign_keys<T: Model>() -> Vec<DDLEntity> {
    let relation_defs = T::relation_definitions();

    let mut entities = vec![];

    for def in relation_defs.iter() {
        match &def.reference {
            Reference::Via((junction_table, from_ref, to_ref)) => {
                let from_table = T::table_name();
                let from_table_id_field = T::id_field_name();
                let to_table = (def.model_definition.table_name)();
                let to_table_id_field = (def.model_definition.id_field_name)();

                let columns = format!("{} UUID NOT NULL, {} UUID NOT NULL", from_ref, to_ref);
                let primary_key = format!("PRIMARY KEY ({}, {})", from_ref, to_ref);
                let from_foreign_key_constraint = format!(
                    "CONSTRAINT fk_from_reference FOREIGN KEY ({}) REFERENCES {} ({})",
                    from_ref, from_table, from_table_id_field
                );
                let to_foreign_key_constraint = format!(
                    "CONSTRAINT fk_to_reference FOREIGN KEY ({}) REFERENCES {} ({})",
                    to_ref, to_table, to_table_id_field
                );

                let create_statement = format!(
                    "CREATE TABLE {} ({}, {}, {}, {});",
                    junction_table,
                    columns,
                    primary_key,
                    from_foreign_key_constraint,
                    to_foreign_key_constraint
                );

                entities.push(DDLEntity::JunctionTable((
                    junction_table.into(),
                    create_statement,
                )))
            }
            Reference::From(column) => {
                let table = T::table_name();
                let foreign_table = (def.model_definition.table_name)();
                let foreign_column = (def.model_definition.id_field_name)();

                let alter_statement = format!(
                    "ALTER TABLE {} ADD CONSTRAINT fk_{} FOREIGN KEY ({}) REFERENCES {} ({});",
                    table, def.name, column, foreign_table, foreign_column
                );

                entities.push(DDLEntity::ForeignKeyConstraint((
                    format!("{}_{}", table, def.name),
                    alter_statement,
                )))
            }
            _ => {}
        }
    }

    entities
}

fn create_indices<T: Model>() -> Vec<DDLEntity> {
    let indices = T::index_definitions();

    indices
        .into_iter()
        .map(|def| {
            let table_name = T::table_name();
            let idx_name = format!("idx_{}_{}", table_name, def.name);

            let index_type = match def.type_ {
                IndexType::BTree => "btree",
                IndexType::Fulltext | IndexType::Trigram => "gin",
            };

            let columns_string = match def.type_ {
                IndexType::BTree => def.columns.join(", "),

                IndexType::Fulltext => def
                    .columns
                    .iter()
                    .map(|c| format!("to_tsvector('english', {}::text)", c))
                    .collect::<Vec<_>>()
                    .join(", "),

                IndexType::Trigram => def
                    .columns
                    .iter()
                    .map(|c| format!("{}::text gin_trgm_ops", c))
                    .collect::<Vec<_>>()
                    .join(", "),
            };

            let statement = format!(
                "CREATE INDEX {} ON {} USING {} ({});",
                idx_name, table_name, index_type, columns_string,
            );

            DDLEntity::Index((idx_name, statement))
        })
        .collect()
}
