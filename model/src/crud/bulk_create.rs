use sqlx::PgConnection;

use crate::Error;
use crate::Model;

pub struct BulkCreate<'a, T: Model> {
    iterator: Box<dyn Iterator<Item = T> + 'a>,
}

impl<'a, T: Model> BulkCreate<'a, T> {
    pub(crate) fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = T> + 'a,
    {
        Self {
            iterator: Box::new(iter),
        }
    }

    pub async fn execute(self, executor: &mut PgConnection) -> Result<(), Error> {
        let table_name = T::table_name();
        let fields = T::field_definitions()
            .into_iter()
            .map(|field| format!("\"{}\"", field.name))
            .collect::<Vec<String>>()
            .join(", ");

        let statement = format!(
            "COPY {} ({}) FROM stdin WITH (FORMAT csv, HEADER false, NULL 'null')",
            table_name, fields
        );

        let mut writer = executor.copy_in_raw(&statement).await?;

        for record in self.iterator {
            let values = T::field_definitions()
                .into_iter()
                .map(|def| record.field_value(&def.name).unwrap().to_string())
                .collect::<Vec<String>>()
                .join(",");

            let row = format!("{}\n", values);
            writer.send(row.as_bytes()).await?;
        }

        writer.finish().await?;

        Ok(())
    }
}
