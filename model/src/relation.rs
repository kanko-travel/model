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

    pub fn to_join_clause(&self, parent: &str, parent_id_column: &str, is_root: bool) -> String {
        let related_table = (self.model_definition.table_name)();
        let related_id_column = (self.model_definition.id_field_name)();

        let alias = if is_root {
            self.name.clone()
        } else {
            format!("{}_{}", parent, self.name)
        };

        match &self.reference {
            Reference::From(column) => {
                format!(
                    "LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                    related_table, alias, parent, column, alias, related_id_column
                )
            }
            Reference::To(column) => {
                format!(
                    "LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                    related_table, alias, parent, parent_id_column, alias, column,
                )
            }
            Reference::Via((junction_table, from_reference, to_reference)) => {
                let join_junction = format!(
                    "LEFT JOIN {} AS {}_{} ON {}.{} = {}_{}.{}",
                    junction_table,
                    parent,
                    junction_table,
                    parent,
                    parent_id_column,
                    parent,
                    junction_table,
                    from_reference,
                );

                let join_relation = format!(
                    "INNER JOIN {} AS {} ON {}_{}.{} = {}.{}",
                    related_table,
                    alias,
                    parent,
                    junction_table,
                    to_reference,
                    alias,
                    related_id_column,
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
    use crate::{self as model, schema};

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
                    "registered_students".into(),
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

        let ddl = schema!(Student, Course, Dorm);

        println!("{}", ddl);
    }
}
