use crate::{FieldDefinition, Model};

pub struct RelationDef {
    pub identity: RelationIdent,
    pub target: RelationTarget,
}

pub enum RelationIdent {
    BelongsTo { name: String, reference: String },
    HasOne { name: String, reference: String },
    HasMany { name: String, reference: String },
}

pub struct RelationTarget {
    pub table_name: fn() -> String,
    pub id_field_name: fn() -> String,
    pub field_definitions: fn() -> Vec<FieldDefinition>,
}

pub trait Relation<T: Model> {
    fn identity() -> RelationIdent;

    fn def() -> RelationDef {
        RelationDef {
            identity: Self::identity(),
            target: RelationTarget {
                table_name: T::table_name,
                id_field_name: T::id_field_name,
                field_definitions: T::field_definitions,
            },
        }
    }
}
