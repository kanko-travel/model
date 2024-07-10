use crate::field_value::FieldValue;
use crate::util::to_b64_str;
use crate::Error;
use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject},
    JsonSchema,
};
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Cursor {
    pub value: Option<FieldValue>,
    pub id: Uuid,
}

impl Serialize for Cursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut data = to_b64_str(&self.id).map_err(serde::ser::Error::custom)?;
        if let Some(value) = &self.value {
            let value = match value {
                FieldValue::Uuid(v) => to_b64_str(v),
                FieldValue::Bool(v) => to_b64_str(v),
                FieldValue::Int(v) => to_b64_str(v),
                FieldValue::Int32(v) => to_b64_str(v),
                FieldValue::Float(v) => to_b64_str(v),
                FieldValue::Decimal(v) => to_b64_str(v),
                FieldValue::String(v) => to_b64_str(v),
                FieldValue::Date(v) => to_b64_str(v),
                FieldValue::DateTime(v) => to_b64_str(v),
                FieldValue::Enum(v) => to_b64_str(v),
                FieldValue::Json(_) => {
                    return Err(serde::ser::Error::custom(Error::bad_request(
                        "can't serialize cursor that has a json value",
                    )))
                }
            }
            .map_err(serde::ser::Error::custom)?;

            data = format!("{}_{}", &data, &value);
        }

        serializer.serialize_str(&data)
    }
}

impl JsonSchema for Cursor {
    fn schema_name() -> String {
        // Exclude the module path to make the name in generated schemas clearer.
        "Cursor".to_owned()
    }

    fn schema_id() -> Cow<'static, str> {
        // Include the module, in case a type with the same name is in another module/crate
        // Cow::Borrowed(concat!(module_path!(), "::Cursor"))
        Cow::Owned(format!("{}::Cursor", module_path!()))
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
        .into()
    }

    fn is_referenceable() -> bool {
        false
    }
}
