use crate::Cursor;

use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::Serialize;
use std::borrow::Cow;

#[derive(Clone, Debug, Serialize, JsonSchema)]
struct ConnectionWithSchema<T: JsonSchema> {
    pub nodes: Vec<T>,
    pub page_info: PageInfo,
}

#[derive(Clone, Debug, Serialize)]
pub struct Connection<T> {
    pub nodes: Vec<T>,
    pub page_info: PageInfo,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct PageInfo {
    pub next_cursor: Option<Cursor>,
}

impl<T: JsonSchema> JsonSchema for Connection<T> {
    fn schema_name() -> String {
        // Exclude the module path to make the name in generated schemas clearer.
        format!("Connection_{}", T::schema_name())
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Owned(format!(
            "{}::Connection<{}>",
            module_path!(),
            T::schema_id()
        ))
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        ConnectionWithSchema::<T>::json_schema(gen)
    }
}
