//! Compares the rmcp-generated tool list against the legacy `tools_baseline.json` fixture
//! (tool names, descriptions and top-level required fields).
//!
//! Intentional schema differences from the legacy hand-written schemas (schemars output):
//! - nested types are emitted as `$defs` + `$ref` instead of inlined objects;
//! - optional fields may be typed as nullable (e.g. `["string", "null"]`) or use `anyOf`;
//! - extra keywords appear: `format` (e.g. `date-time`, `double`), `minimum`, and `$schema`;
//! - `RepRangeParams.start`/`end` are required (legacy schema listed no required fields there);
//! - `create_routine_folder` accepts an undocumented `index`, hidden from the schema.

use std::collections::{BTreeMap, BTreeSet};

use hevy_mcp_server::HevyServer;
use serde_json::Value;

fn required_of(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn tool_list_matches_baseline() {
    let baseline: Value =
        serde_json::from_str(include_str!("fixtures/tools_baseline.json")).unwrap();
    let expected: BTreeMap<String, (String, BTreeSet<String>)> = baseline["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().to_string(),
                (
                    t["description"].as_str().unwrap().to_string(),
                    required_of(&t["inputSchema"]),
                ),
            )
        })
        .collect();
    assert_eq!(expected.len(), 17);

    let actual: BTreeMap<String, (String, BTreeSet<String>)> = HevyServer::tool_router()
        .list_all()
        .into_iter()
        .map(|t| {
            let schema = Value::Object((*t.input_schema).clone());
            (
                t.name.to_string(),
                (
                    t.description.map(|d| d.to_string()).unwrap_or_default(),
                    required_of(&schema),
                ),
            )
        })
        .collect();

    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "tool names differ"
    );
    for (name, exp) in &expected {
        assert_eq!(
            &actual[name], exp,
            "description/required mismatch for {name}"
        );
    }
}
