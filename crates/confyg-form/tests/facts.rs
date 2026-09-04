use confyg_form::facts::{facts, AdditionalProperties};

#[test]
fn reads_the_eight_keywords_hints_edit_ignores() {
    let s = serde_json::json!({
        "type": "integer", "default": 8080, "examples": [80, 443],
        "readOnly": true, "deprecated": true, "minimum": 1, "maximum": 65535,
        "multipleOf": 1
    });
    let f = facts(&s);
    assert_eq!(f.default, Some(serde_json::json!(8080)));
    assert_eq!(f.examples.len(), 2);
    assert!(f.read_only && f.deprecated);
    assert_eq!((f.bounds.min, f.bounds.max), (Some(1.0), Some(65535.0)));
}

#[test]
fn additional_properties_has_three_forms() {
    let obj = serde_json::json!({"additionalProperties": {"type": "string"}});
    assert!(matches!(
        facts(&obj).additional,
        AdditionalProperties::Schema(_)
    ));
    assert!(matches!(
        facts(&serde_json::json!({})).additional,
        AdditionalProperties::Open
    ));
    assert!(matches!(
        facts(&serde_json::json!({"additionalProperties": false})).additional,
        AdditionalProperties::Closed
    ));
}

#[test]
fn property_order_follows_the_schema_file() {
    let s: serde_json::Value =
        serde_json::from_str(r#"{"properties":{"host":{},"port":{},"ca_cert":{}}}"#).unwrap();
    let keys: Vec<_> = s["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        keys,
        ["host", "port", "ca_cert"],
        "preserve_order is not enabled (D6)"
    );
}

#[test]
fn a_malformed_schema_reads_as_absent_never_panics() {
    let s = serde_json::json!({
        "type": 7, "default": null, "examples": "not an array",
        "minimum": "low", "required": "host", "enum": 3, "pattern": 9
    });
    let f = facts(&s);
    assert!(f.ty.is_none());
    assert_eq!(f.default, Some(serde_json::Value::Null));
    assert!(f.examples.is_empty());
    assert_eq!(f.bounds.min, None);
    assert!(f.required.is_empty());
    assert!(f.enum_values.is_none());
    assert!(f.pattern.is_none());
}
