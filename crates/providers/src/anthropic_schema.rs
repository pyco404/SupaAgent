//! Adapt only application-owned schemas. Rust validators retain every original constraint.
use super::*;
pub(super) fn wire_schema(schema: &Value) -> Result<Value> {
    fn visit(schema: &Value, depth: usize) -> Result<Value> {
        if depth > 32 {
            return Err(Error::ProviderRequest);
        }
        let mut object = schema.as_object().ok_or(Error::ProviderRequest)?.clone();
        if object.contains_key("$ref") {
            return Err(Error::ProviderRequest);
        }
        let mut constraints = vec![];
        for key in [
            "minimum",
            "maximum",
            "exclusiveMinimum",
            "exclusiveMaximum",
            "multipleOf",
            "minLength",
            "maxLength",
            "maxItems",
            "uniqueItems",
        ] {
            if let Some(value) = object.remove(key) {
                constraints.push(format!("{key}={value}"));
            }
        }
        if object
            .get("minItems")
            .is_some_and(|n| n.as_u64().is_none_or(|n| n > 1))
        {
            let value = object.remove("minItems").unwrap();
            constraints.push(format!("minItems={value}"));
        }
        if !constraints.is_empty() {
            let prefix = object
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            object.insert(
                "description".into(),
                json!(format!(
                    "{prefix} Application-enforced constraints: {}.",
                    constraints.join(", ")
                )),
            );
        }
        if let Some(properties) = object.get_mut("properties") {
            for value in properties
                .as_object_mut()
                .ok_or(Error::ProviderRequest)?
                .values_mut()
            {
                *value = visit(value, depth + 1)?;
            }
        }
        if let Some(items) = object.get_mut("items") {
            *items = visit(items, depth + 1)?;
        }
        for key in ["anyOf", "allOf"] {
            if let Some(branches) = object.get_mut(key) {
                for value in branches.as_array_mut().ok_or(Error::ProviderRequest)? {
                    *value = visit(value, depth + 1)?;
                }
            }
        }
        Ok(Value::Object(object))
    }
    visit(schema, 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_constraints_move_to_descriptions_without_erasing_property_names() {
        let original = json!({"type":"object","additionalProperties":false,"properties":{"minimum":{"type":"integer","minimum":1},"rows":{"type":"array","minItems":1,"maxItems":3,"items":{"type":"string","maxLength":10}}},"required":["minimum","rows"]});
        let adapted = wire_schema(&original).unwrap();
        assert!(adapted["properties"].get("minimum").is_some());
        assert!(adapted["properties"]["minimum"].get("minimum").is_none());
        assert_eq!(adapted["properties"]["rows"]["minItems"], 1);
        assert!(adapted["properties"]["rows"].get("maxItems").is_none());
        assert_eq!(original["properties"]["minimum"]["minimum"], 1);
        assert_eq!(adapted["additionalProperties"], false);
    }
}
