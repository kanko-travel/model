use std::{fs::File, io::BufReader, path::Path};

use model::{schema, Crud, Model};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, Model, FromRow)]
#[model(table_name = "dummy")]
struct Dummy {
    #[model(id)]
    id: Uuid,
    #[model(primary_key)]
    name: Option<String>,
    #[model(primary_key)]
    age: Option<i64>,
}

async fn create_db_pool() -> PgPool {
    PgPool::connect("postgresql://model:model@postgres-model:5432/model")
        .await
        .unwrap()
}

async fn setup_tables(tx: &mut Transaction<'_, Postgres>) {
    let ddl = schema!(Dummy);

    for part in ddl.split("\n\n") {
        sqlx::query(part)
            .execute(tx as &mut PgConnection)
            .await
            .unwrap();
    }
}

fn read_records() -> Vec<Dummy> {
    let path = Path::new("./tests/dummy_records.json");

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).unwrap()
}

#[tokio::test]
async fn test_create() {
    let pool = create_db_pool().await;

    let mut tx = pool.begin().await.unwrap();

    setup_tables(&mut tx).await;

    let record = Dummy {
        id: Uuid::new_v4(),
        age: 24.into(),
        name: "John Doe".to_string().into(),
    };

    record.create().execute(&mut tx).await.unwrap();

    let inserted = Dummy::select()
        .by_id(record.id.clone())
        .fetch_one(&mut tx)
        .await
        .unwrap();

    assert_eq!(inserted.id, record.id);
    assert_eq!(inserted.name, record.name);
    assert_eq!(inserted.age, record.age);

    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn test_upsert() {
    let pool = create_db_pool().await;

    let mut tx = pool.begin().await.unwrap();

    setup_tables(&mut tx).await;

    let records = read_records();

    let mut records = records.into_iter();

    let record = records.next().unwrap();

    record.create().execute(&mut tx).await.unwrap();

    let mut upserted = record.clone();
    upserted.id = Uuid::new_v4();

    upserted.upsert().execute(&mut tx).await.unwrap();

    assert_eq!(upserted.id, record.id);
    assert_eq!(upserted.name, record.name);
    assert_eq!(upserted.age, record.age);

    tx.rollback().await.unwrap();
}
