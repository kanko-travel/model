use crate::{FieldDefinition, Model};

pub trait Relation<T: Model> {
    fn identity() -> RelationIdent;

    fn def() -> RelationDef {
        RelationDef {
            identity: Self::identity(),
            related_model: RelatedModel {
                table_name: T::table_name,
                id_field_name: T::id_field_name,
                field_definitions: T::field_definitions,
            },
        }
    }
}

pub struct RelationDef {
    pub identity: RelationIdent,
    pub related_model: RelatedModel,
}

pub enum RelationIdent {
    BelongsTo { name: String, from: String },
    HasOne { name: String, to: String },
    HasMany { name: String, to: String },
    HasManyVia { name: String, via: String, from: String, to: String }
}

pub struct RelatedModel {
    pub table_name: fn() -> String,
    pub id_field_name: fn() -> String,
    pub field_definitions: fn() -> Vec<FieldDefinition>,
}
