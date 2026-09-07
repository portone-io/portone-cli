use serde_json::{Map, Value, json};

#[derive(Debug, PartialEq, Eq)]
pub enum Advance {
    Next,
    Done,
    Stop(String),
}

pub struct Paginator {
    number: u64,
    size: u64,
    cursor_size: Option<u64>,
    injected_page: bool,
}

impl Paginator {
    pub fn new(params: &mut Map<String, Value>) -> Paginator {
        if let Some(page) = params.get("page") {
            let number = page.get("number").and_then(Value::as_u64).unwrap_or(0);
            let size = page.get("size").and_then(Value::as_u64).unwrap_or(100);
            Paginator {
                number,
                size,
                cursor_size: None,
                injected_page: false,
            }
        } else if params.contains_key("size") || params.contains_key("cursor") {
            if !params.contains_key("size") {
                params.insert("size".to_string(), json!(1000));
            }
            let cursor_size = params.get("size").and_then(Value::as_u64);
            Paginator {
                number: 0,
                size: 100,
                cursor_size,
                injected_page: false,
            }
        } else {
            params.insert("page".to_string(), json!({"number": 0, "size": 100}));
            Paginator {
                number: 0,
                size: 100,
                cursor_size: None,
                injected_page: true,
            }
        }
    }

    pub fn advance(&mut self, params: &mut Map<String, Value>, body: &Value) -> Advance {
        let total_count = body
            .get("page")
            .and_then(|p| p.get("totalCount"))
            .and_then(Value::as_u64);
        let items = body.get("items").and_then(Value::as_array);

        if let Some(total) = total_count {
            self.advance_offset(params, body, total, items)
        } else if let Some(items) = items {
            self.advance_cursor(params, items)
        } else {
            Advance::Stop(
                "cannot determine pagination scheme; stopping after first page".to_string(),
            )
        }
    }

    fn advance_offset(
        &mut self,
        params: &mut Map<String, Value>,
        body: &Value,
        total: u64,
        items: Option<&Vec<Value>>,
    ) -> Advance {
        let size = body
            .get("page")
            .and_then(|p| p.get("size"))
            .and_then(Value::as_u64)
            .unwrap_or(self.size);
        if size == 0 {
            return Advance::Done;
        }
        if items.is_none_or(|items| items.is_empty()) {
            return Advance::Done;
        }
        let fetched = (self.number + 1) * size;
        if fetched >= total {
            return Advance::Done;
        }
        if fetched + size > 60000 {
            return Advance::Stop("offset pagination limit (60000) reached; stopping".to_string());
        }

        self.number += 1;
        self.size = size;
        let page = params
            .entry("page".to_string())
            .or_insert_with(|| json!({}));
        if !page.is_object() {
            *page = json!({});
        }
        let obj = page
            .as_object_mut()
            .expect("page was normalized to an object");
        obj.insert("number".to_string(), json!(self.number));
        obj.insert("size".to_string(), json!(self.size));
        Advance::Next
    }

    fn advance_cursor(&mut self, params: &mut Map<String, Value>, items: &[Value]) -> Advance {
        if self.injected_page {
            params.remove("page");
            self.injected_page = false;
        }
        if items.is_empty() {
            return Advance::Done;
        }
        if let Some(requested) = self.cursor_size
            && (items.len() as u64) < requested
        {
            return Advance::Done;
        }
        let Some(cursor) = items
            .last()
            .and_then(|item| item.get("cursor"))
            .and_then(Value::as_str)
        else {
            return Advance::Done;
        };
        params.insert("cursor".to_string(), json!(cursor));
        Advance::Next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn new_injects_offset_defaults_without_hint() {
        let mut params = Map::new();
        Paginator::new(&mut params);
        assert_eq!(params.get("page"), Some(&json!({"number": 0, "size": 100})));
    }

    #[test]
    fn new_keeps_existing_page_params() {
        let mut params = map(json!({"page": {"number": 3, "size": 20}}));
        Paginator::new(&mut params);
        assert_eq!(params.get("page"), Some(&json!({"number": 3, "size": 20})));
    }

    #[test]
    fn new_injects_cursor_size_when_only_cursor_given() {
        let mut params = map(json!({"cursor": "abc"}));
        Paginator::new(&mut params);
        assert_eq!(params.get("size"), Some(&json!(1000)));
    }

    #[test]
    fn offset_advance_increments_number_and_uses_response_size() {
        let mut params = Map::new();
        let mut p = Paginator::new(&mut params);
        let body = json!({"items": [1, 2], "page": {"number": 0, "size": 50, "totalCount": 120}});
        assert_eq!(p.advance(&mut params, &body), Advance::Next);
        assert_eq!(params.get("page"), Some(&json!({"number": 1, "size": 50})));

        let body = json!({"items": [1], "page": {"number": 1, "size": 50, "totalCount": 120}});
        assert_eq!(p.advance(&mut params, &body), Advance::Next);
        assert_eq!(params.get("page"), Some(&json!({"number": 2, "size": 50})));

        let body = json!({"items": [1], "page": {"number": 2, "size": 50, "totalCount": 120}});
        assert_eq!(p.advance(&mut params, &body), Advance::Done);
    }

    #[test]
    fn offset_advance_stops_on_empty_items() {
        let mut params = Map::new();
        let mut p = Paginator::new(&mut params);
        let body = json!({"items": [], "page": {"size": 100, "totalCount": 500}});
        assert_eq!(p.advance(&mut params, &body), Advance::Done);
    }

    #[test]
    fn offset_advance_guards_60000_limit() {
        let mut params = map(json!({"page": {"number": 1, "size": 30000}}));
        let mut p = Paginator::new(&mut params);
        let body =
            json!({"items": [1], "page": {"number": 1, "size": 30000, "totalCount": 1000000}});
        match p.advance(&mut params, &body) {
            Advance::Stop(msg) => assert!(msg.contains("60000")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn offset_advance_allows_next_request_up_to_60000() {
        let mut params = map(json!({"page": {"number": 0, "size": 30000}}));
        let mut p = Paginator::new(&mut params);
        let body =
            json!({"items": [1], "page": {"number": 0, "size": 30000, "totalCount": 1000000}});
        assert_eq!(p.advance(&mut params, &body), Advance::Next);
    }

    #[test]
    fn cursor_advance_sets_cursor_from_last_item() {
        let mut params = map(json!({"size": 2}));
        let mut p = Paginator::new(&mut params);
        let body = json!({"items": [{"cursor": "a"}, {"cursor": "b"}]});
        assert_eq!(p.advance(&mut params, &body), Advance::Next);
        assert_eq!(params.get("cursor"), Some(&json!("b")));
    }

    #[test]
    fn cursor_advance_stops_when_short_page() {
        let mut params = map(json!({"size": 2}));
        let mut p = Paginator::new(&mut params);
        let body = json!({"items": [{"cursor": "a"}]});
        assert_eq!(p.advance(&mut params, &body), Advance::Done);
    }

    #[test]
    fn cursor_advance_stops_without_cursor_field() {
        let mut params = map(json!({"size": 1}));
        let mut p = Paginator::new(&mut params);
        let body = json!({"items": [{"id": 1}]});
        assert_eq!(p.advance(&mut params, &body), Advance::Done);
    }

    #[test]
    fn cursor_detection_removes_injected_page() {
        let mut params = Map::new();
        let mut p = Paginator::new(&mut params);
        assert!(params.contains_key("page"));
        let body = json!({"items": [{"cursor": "a"}]});
        assert_eq!(p.advance(&mut params, &body), Advance::Next);
        assert!(!params.contains_key("page"));
        assert_eq!(params.get("cursor"), Some(&json!("a")));
    }

    #[test]
    fn unknown_scheme_warns_and_stops() {
        let mut params = Map::new();
        let mut p = Paginator::new(&mut params);
        let body = json!({"ok": true});
        assert_eq!(
            p.advance(&mut params, &body),
            Advance::Stop(
                "cannot determine pagination scheme; stopping after first page".to_string()
            )
        );
    }
}
