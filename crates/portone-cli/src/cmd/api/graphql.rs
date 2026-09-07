use serde_json::{Map, Value};

pub fn group_variables(params: &Map<String, Value>) -> Map<String, Value> {
    let mut top = Map::new();
    let mut variables = Map::new();
    for (key, value) in params {
        match key.as_str() {
            "query" | "operationName" => {
                top.insert(key.clone(), value.clone());
            }
            _ => {
                variables.insert(key.clone(), value.clone());
            }
        }
    }
    if !variables.is_empty() {
        top.insert("variables".to_string(), Value::Object(variables));
    }
    top
}

pub fn find_end_cursor(body: &Value) -> Option<String> {
    let page_info = find_page_info(body)?;
    if page_info.get("hasNextPage").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    page_info
        .get("endCursor")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn find_page_info(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "pageInfo"
                    && let Some(obj) = child.as_object()
                {
                    return Some(obj);
                }
                if let Some(found) = find_page_info(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_page_info),
        _ => None,
    }
}

pub fn error_message(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let errors = value.as_object()?.get("errors")?.as_array()?;
    let messages: Vec<&str> = errors
        .iter()
        .filter_map(|err| match err {
            Value::String(text) => Some(text.as_str()),
            Value::Object(obj) => obj.get("message").and_then(Value::as_str),
            _ => None,
        })
        .collect();
    if messages.is_empty() {
        return None;
    }
    Some(messages.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn group_variables_keeps_empty_map_empty() {
        assert_eq!(group_variables(&Map::new()), Map::new());
    }

    #[test]
    fn group_variables_keeps_query_top_level_without_variables() {
        let grouped = group_variables(&map(json!({"query": "QUERY"})));
        assert_eq!(grouped, map(json!({"query": "QUERY"})));
    }

    #[test]
    fn group_variables_moves_fields_without_query() {
        let grouped = group_variables(&map(json!({"name": "hubot"})));
        assert_eq!(grouped, map(json!({"variables": {"name": "hubot"}})));
    }

    #[test]
    fn group_variables_preserves_value_types() {
        let grouped = group_variables(&map(json!({
            "query": "QUERY",
            "name": "hubot",
            "power": 9001,
        })));
        assert_eq!(
            grouped,
            map(json!({
                "query": "QUERY",
                "variables": {"name": "hubot", "power": 9001},
            }))
        );
    }

    #[test]
    fn group_variables_keeps_operation_name_top_level() {
        let grouped = group_variables(&map(json!({
            "query": "QUERY",
            "operationName": "Op",
            "power": 9001,
        })));
        assert_eq!(
            grouped,
            map(json!({
                "query": "QUERY",
                "operationName": "Op",
                "variables": {"power": 9001},
            }))
        );
    }

    #[test]
    fn group_variables_nests_explicit_variables_key() {
        let grouped = group_variables(&map(json!({"variables": {"name": "x"}})));
        assert_eq!(
            grouped,
            map(json!({"variables": {"variables": {"name": "x"}}}))
        );
    }

    #[test]
    fn find_end_cursor_returns_none_for_empty_object() {
        assert_eq!(find_end_cursor(&json!({})), None);
    }

    #[test]
    fn find_end_cursor_ignores_fields_outside_page_info() {
        let body = json!({"hasNextPage": true, "endCursor": "NOPE"});
        assert_eq!(find_end_cursor(&body), None);
    }

    #[test]
    fn find_end_cursor_extracts_cursor_from_page_info() {
        let body = json!({
            "data": {"nodes": [], "pageInfo": {"hasNextPage": true, "endCursor": "THE_END"}}
        });
        assert_eq!(find_end_cursor(&body), Some("THE_END".to_string()));
    }

    #[test]
    fn find_end_cursor_uses_first_page_info_in_document_order() {
        let body = json!({
            "a": {"pageInfo": {"hasNextPage": true, "endCursor": "THE_END"}},
            "pageInfo": {"hasNextPage": true, "endCursor": "NOT_THIS"},
        });
        assert_eq!(find_end_cursor(&body), Some("THE_END".to_string()));
    }

    #[test]
    fn find_end_cursor_returns_none_when_no_next_page() {
        let body = json!({"pageInfo": {"hasNextPage": false, "endCursor": "THE_END"}});
        assert_eq!(find_end_cursor(&body), None);
    }

    #[test]
    fn find_end_cursor_returns_none_for_null_cursor() {
        let body = json!({"pageInfo": {"hasNextPage": true, "endCursor": null}});
        assert_eq!(find_end_cursor(&body), None);
    }

    #[test]
    fn find_end_cursor_requires_boolean_has_next_page() {
        let body = json!({"pageInfo": {"hasNextPage": "true", "endCursor": "THE_END"}});
        assert_eq!(find_end_cursor(&body), None);
    }

    #[test]
    fn error_message_joins_object_messages() {
        let body = br#"{"errors":[{"message":"fail1"},{"message":"asplode2"}]}"#;
        assert_eq!(error_message(body), Some("fail1\nasplode2".to_string()));
    }

    #[test]
    fn error_message_joins_string_errors() {
        let body = br#"{"errors":["a","b"]}"#;
        assert_eq!(error_message(body), Some("a\nb".to_string()));
    }

    #[test]
    fn error_message_ignores_null_errors() {
        assert_eq!(error_message(br#"{"errors":null}"#), None);
    }

    #[test]
    fn error_message_ignores_empty_errors() {
        assert_eq!(error_message(br#"{"errors":[]}"#), None);
    }

    #[test]
    fn error_message_ignores_missing_or_invalid_bodies() {
        assert_eq!(error_message(br#"{"data":{}}"#), None);
        assert_eq!(error_message(b"not json"), None);
        assert_eq!(error_message(br#"[{"message":"x"}]"#), None);
    }
}
