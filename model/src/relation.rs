use crate::{Model, ModelDef};

pub trait Related {
    fn relation_definitions() -> Vec<RelationDef> {
        vec![]
    }
}

#[derive(Debug)]
pub struct RelationDef {
    pub name: String,
    pub reference: Reference,
    pub model_definition: ModelDef,
}

#[derive(Debug)]
pub enum Reference {
    Direct(ReferenceDirect),
    Via(ReferenceVia),
}

#[derive(Debug)]
pub struct ReferenceDirect {
    pub from: (String, String),
    pub to: (String, String),
}

#[derive(Debug)]
pub struct ReferenceVia {
    pub from: (String, String),
    pub via: (String, String, String),
    pub to: (String, String),
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

#[cfg(test)]
mod test {
    use crate as model;

    use crate::Model;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_relations() {
        #[derive(Clone, Debug, Model)]
        #[model(table_name = "students", has_relations)]
        struct Student {
            #[model(primary_key, id)]
            id: Uuid,
            name: String,
            dorm_id: Uuid,
        }

        #[derive(Clone, Debug, Model)]
        #[model(table_name = "courses", has_relations)]
        struct Course {
            #[model(primary_key, id)]
            id: Uuid,
            name: String,
        }

        #[derive(Clone, Debug, Model)]
        #[model(table_name = "dorms", has_relations)]
        struct Dorm {
            #[model(primary_key, id)]
            id: Uuid,
            name: String,
        }

        impl Related for Student {
            fn relation_definitions() -> Vec<RelationDef> {
                vec![
                    Self::has_many_via_junction_table::<Course>(
                        "registered_courses".into(),
                        "student_registered_courses".into(),
                    ),
                    Self::belongs_to::<Dorm>("dorm".into(), "dorm_id".into()),
                ]
            }
        }

        impl Related for Course {
            fn relation_definitions() -> Vec<RelationDef> {
                vec![Self::has_many_via_junction_table::<Student>(
                    "students".into(),
                    "student_registered_courses".into(),
                )]
            }
        }

        impl Related for Dorm {
            fn relation_definitions() -> Vec<RelationDef> {
                vec![Self::has_many::<Student>(
                    "students".into(),
                    "dorm_id".into(),
                )]
            }
        }

        let student_relations = Student::relation_definitions();
        let course_relations = Course::relation_definitions();
        let dorm_relations = Dorm::relation_definitions();

        println!("{:?}", student_relations);
        println!("{:?}", course_relations);
        println!("{:?}", dorm_relations);
    }
}
