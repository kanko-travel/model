use std::{fs::File, io::BufReader, path::Path, str::FromStr};

use model::{Crud, Cursor, Filter, Migration, Model, Query, Sort};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, Model, FromRow)]
#[model(table_name = "dummy")]
struct Dummy {
    #[model(id, primary_key)]
    id: Uuid,
    name: Option<String>,
    age: Option<i64>,
}

async fn create_db_pool() -> PgPool {
    PgPool::connect("postgresql://model:model@postgres-model:5432/model")
        .await
        .unwrap()
}

async fn setup_tables(tx: &mut Transaction<'_, Postgres>) {
    Dummy::migrate(tx).await.unwrap();
}

async fn insert_records(tx: &mut Transaction<'_, Postgres>) {
    let records = read_records();

    for record in records.into_iter() {
        record.create().execute(tx).await.unwrap();
    }
}

fn read_records() -> Vec<Dummy> {
    let path = Path::new("./tests/dummy_records.json");

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);

    serde_json::from_reader(reader).unwrap()
}

#[tokio::test]
async fn test_pagination() {
    let pool = create_db_pool().await;

    let mut tx = pool.begin().await.unwrap();

    setup_tables(&mut tx).await;
    insert_records(&mut tx).await;

    let mut query = Query::new();
    query.limit = 2.into();
    query.filter = Filter::new()
        .field("age")
        .gte(28)
        .build::<Dummy>()
        .unwrap()
        .into();
    query.sort = Sort {
        field: "age".into(),
        direction: model::SortDirection::Ascending,
    }
    .into();

    let connection = Dummy::select()
        .from_query(query.clone())
        .unwrap()
        .fetch_page(&mut tx)
        .await
        .unwrap();

    assert_eq!(connection.page_info.prev_cursor, None);
    assert_eq!(
        connection.page_info.next_cursor,
        Some(Cursor {
            id: Uuid::from_str("069f0ae7-e354-4099-83c7-4c20616f8d95").unwrap(),
            value: Some(model::FieldValue::Int(30.into())),
        })
    );

    let mut nodes = connection.nodes.into_iter();
    let node = nodes.next().unwrap();

    assert_eq!(node.name, Some("Mark".to_string()));

    let node = nodes.next().unwrap();

    assert_eq!(node.name, Some("Noble".to_string()));

    query.cursor = connection.page_info.next_cursor;

    let connection = Dummy::select()
        .from_query(query.clone())
        .unwrap()
        .fetch_page(&mut tx)
        .await
        .unwrap();

    assert_eq!(
        connection.page_info.prev_cursor,
        Some(Cursor {
            id: Uuid::from_str("069f0ae7-e354-4099-83c7-4c20616f8d98").unwrap(),
            value: Some(model::FieldValue::Int(28.into())),
        })
    );
    assert_eq!(
        connection.page_info.next_cursor,
        Some(Cursor {
            id: Uuid::from_str("069f0ae7-e354-4099-83c7-4c20616f8d97").unwrap(),
            value: Some(model::FieldValue::Int(31.into())),
        })
    );

    let mut nodes = connection.nodes.into_iter();
    let node = nodes.next().unwrap();

    assert_eq!(node.name, Some("Kendra".to_string()));

    let node = nodes.next().unwrap();

    assert_eq!(node.name, Some("Kerry".to_string()));

    query.cursor = connection.page_info.next_cursor;

    let connection = Dummy::select()
        .from_query(query.clone())
        .unwrap()
        .fetch_page(&mut tx)
        .await
        .unwrap();

    assert_eq!(
        connection.page_info.prev_cursor,
        Some(Cursor {
            id: Uuid::from_str("069f0ae7-e354-4099-83c7-4c20616f8d95").unwrap(),
            value: Some(model::FieldValue::Int(30.into())),
        })
    );
    assert_eq!(connection.page_info.next_cursor, None);

    let mut nodes = connection.nodes.into_iter();
    let node = nodes.next().unwrap();

    assert_eq!(node.name, Some("Lewis".to_string()));

    let next = nodes.next();

    assert!(next.is_none());

    tx.rollback().await.unwrap();
}
