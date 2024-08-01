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
    From(String),
    To(String),
    Via((String, String, String)),
}

impl RelationDef {
    pub fn belongs_to<T, U>(name: String, column: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::From(column),
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
            reference: Reference::To(column),
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
            reference: Reference::To(column),
            model_definition: U::definition(),
        }
    }

    pub fn has_many_via<T, U>(name: String, junction_table_name: String) -> Self
    where
        T: Model,
        U: Model,
    {
        RelationDef {
            name,
            reference: Reference::Via((
                junction_table_name,
                junction_table_column(&T::table_name(), &T::id_field_name()),
                junction_table_column(&U::table_name(), &U::id_field_name()),
            )),
            model_definition: U::definition(),
        }
    }

    pub fn to_join_clause(&self, parent: &str, parent_id_column: &str) -> String {
        let related_table = (self.model_definition.table_name)();
        let related_id_column = (self.model_definition.id_field_name)();

        match &self.reference {
            Reference::From(column) => {
                format!(
                    "LEFT JOIN {} ON {}.{} = {}.{} AS {}_{}",
                    related_table,
                    parent,
                    column,
                    related_table,
                    related_id_column,
                    parent,
                    related_table
                )
            }
            Reference::To(column) => {
                format!(
                    "LEFT JOIN {} ON {}.{} = {}.{} AS {}_{}",
                    related_table,
                    parent,
                    parent_id_column,
                    related_table,
                    column,
                    parent,
                    related_table
                )
            }
            Reference::Via((junction_table, from_reference, to_reference)) => {
                let join_junction = format!(
                    "LEFT JOIN {} ON {}.{} = {}.{} AS {}_{}",
                    junction_table,
                    parent,
                    parent_id_column,
                    junction_table,
                    from_reference,
                    parent,
                    junction_table
                );

                let join_relation = format!(
                    "INNER JOIN {} ON {}_{}.{} = {}.{} AS {}_{}",
                    related_table,
                    parent,
                    junction_table,
                    to_reference,
                    related_table,
                    related_id_column,
                    parent,
                    related_table
                );

                format!(
                    "
                    {}
                    {}
                ",
                    join_junction, join_relation
                )
            }
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
                    Self::has_many_via::<Course>(
                        "registered_courses".into(),
                        "student_registered_courses".into(),
                    ),
                    Self::belongs_to::<Dorm>("dorm".into(), "dorm_id".into()),
                ]
            }
        }

        impl Related for Course {
            fn relation_definitions() -> Vec<RelationDef> {
                vec![Self::has_many_via::<Student>(
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
