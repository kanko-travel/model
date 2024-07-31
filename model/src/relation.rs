use crate::{Model, ModelDef};

pub trait Related {
    fn relation_definitions() -> Vec<RelationDef>;
}

pub struct RelationDef {
    pub name: String,
    pub reference: Reference,
    pub model_definition: ModelDef,
}

pub enum Reference {
    Direct(ReferenceDirect),
    Via(ReferenceVia),
}

pub struct ReferenceDirect {
    from: (String, String),
    to: (String, String),
}

pub struct ReferenceVia {
    from: (String, String),
    via: (String, String, String),
    to: (String, String),
}

impl RelationDef {
    pub fn belongs_to<T, U>(name: String, column: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::Direct(ReferenceDirect {
                from: (T::table_name(), column),
                to: (U::table_name(), U::id_field_name()),
            }),
            model_definition: U::definition(),
        }
    }

    pub fn has_one<T, U>(name: String, column: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::Direct(ReferenceDirect {
                from: (T::table_name(), T::id_field_name()),
                to: (U::table_name(), column),
            }),
            model_definition: U::definition(),
        }
    }

    pub fn has_many<T, U>(name: String, column: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::Direct(ReferenceDirect {
                from: (T::table_name(), T::id_field_name()),
                to: (U::table_name(), column),
            }),
            model_definition: U::definition(),
        }
    }

    pub fn has_many_via_junction_table<T, U>(name: String, junction_table_name: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::Via(ReferenceVia {
                from: (T::table_name(), T::id_field_name()),
                to: (U::table_name(), U::id_field_name()),
                via: (
                    junction_table_name,
                    junction_table_column(&T::table_name(), &T::id_field_name()),
                    junction_table_column(&U::table_name(), &U::id_field_name()),
                ),
            }),
            model_definition: U::definition(),
        }
    }
}

fn junction_table_column(table: &str, column: &str) -> String {
    format!("{}_{}", table, column)
}
