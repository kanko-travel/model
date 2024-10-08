use model::{schema, Model, Related, RelationDef};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn create_db_pool() -> PgPool {
    PgPool::connect("postgresql://model:model@postgres:5432/model")
        .await
        .unwrap()
}

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

#[tokio::test]
async fn test_relations() {
    let pool = create_db_pool().await;
    let mut tx = pool.begin().await.unwrap();

    let ddl = schema!(Dorm, Student, Course);

    for part in ddl.split("\n\n") {
        sqlx::query(&part)
            .execute(&mut tx as &mut PgConnection)
            .await
            .unwrap();
    }

    tx.commit().await.unwrap();
}
