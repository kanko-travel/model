use chrono::{NaiveDate, Utc};
use uuid::Uuid;

mod models {
    use chrono::{DateTime, NaiveDate, Utc};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Serialize, Deserialize, Debug, Clone, model::Model)]
    #[model(table_name = "test")]
    pub struct Test {
        #[model(id, primary_key)]
        id: Uuid,
        date_created: DateTime<Utc>,
        anniversary: NaiveDate,
        balance: Decimal,
    }

    model::model_wrapper!(Test);
}

#[test]
fn test_pgoutput_deserialize() {
    use models::ModelWrapper;

    let table_name = "test";
    let col_names = vec![
        "id".to_string(),
        "date_created".to_string(),
        "anniversary".to_string(),
        "balance".to_string(),
    ];

    let anniversaire = NaiveDate::MIN;
    let balance = rust_decimal::Decimal::from(1000);

    let row = vec![
        Some(Uuid::new_v4().to_string()),
        Some(Utc::now().to_rfc3339()),
        Some(anniversaire.to_string()),
        Some(balance.to_string()),
    ];

    ModelWrapper::from_pgoutput(table_name, &col_names, row)
        .expect("failed to deserialize from pgoutput!");
}
