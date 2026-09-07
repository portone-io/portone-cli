use std::io::Read;

use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value};

pub fn parse_fields(
    raw_fields: &[String],
    magic_fields: &[String],
    stdin: &mut dyn Read,
) -> Result<Map<String, Value>> {
    let mut params = Map::new();
    for f in raw_fields {
        parse_field(&mut params, f, false, stdin)?;
    }
    for f in magic_fields {
        parse_field(&mut params, f, true, stdin)?;
    }
    Ok(params)
}

fn parse_field(
    params: &mut Map<String, Value>,
    f: &str,
    is_magic: bool,
    stdin: &mut dyn Read,
) -> Result<()> {
    let mut value_index = 0usize;
    let mut keystack: Vec<&str> = Vec::new();
    let mut key_start_at = 0usize;
    for (i, r) in f.char_indices() {
        match r {
            '[' => {
                if key_start_at == 0 {
                    keystack.push(&f[..i]);
                }
                key_start_at = i + 1;
            }
            ']' => keystack.push(&f[key_start_at..i]),
            '=' => {
                if key_start_at == 0 {
                    keystack.push(&f[..i]);
                }
                value_index = i + 1;
                break;
            }
            _ => {}
        }
    }

    if keystack.is_empty() {
        bail!("invalid key: {f:?}");
    }

    let key;
    let raw_value: Option<&str>;
    if value_index == 0 {
        if !keystack.last().unwrap().is_empty() {
            bail!("field {f:?} requires a value separated by an '=' sign");
        }
        key = f;
        raw_value = None;
    } else {
        key = &f[..value_index - 1];
        raw_value = Some(&f[value_index..]);
    }

    let value: Option<Value> = match raw_value {
        None => None,
        Some(s) if is_magic => match magic_field_value(s, stdin)
            .map_err(|err| anyhow!("error parsing {key:?} value: {err}"))?
        {
            Value::Null => None,
            v => Some(v),
        },
        Some(s) => Some(Value::String(s.to_string())),
    };

    let mut dest_map: &mut Map<String, Value> = params;
    let mut is_array = false;
    let mut subkey = "";
    for &k in &keystack {
        if k.is_empty() {
            is_array = true;
            continue;
        }
        if !subkey.is_empty() {
            if is_array {
                dest_map = add_params_slice(dest_map, subkey, k)?;
                is_array = false;
            } else {
                dest_map = add_params_map(dest_map, subkey)?;
            }
        }
        subkey = k;
    }

    if is_array {
        match value {
            None => {
                dest_map.insert(subkey.to_string(), Value::Array(Vec::new()));
            }
            Some(v) => match dest_map.get_mut(subkey) {
                Some(Value::Array(arr)) => arr.push(v),
                Some(existing) => {
                    bail!(
                        "expected array type under {subkey:?}, got {}",
                        go_type_name(existing)
                    );
                }
                None => {
                    dest_map.insert(subkey.to_string(), Value::Array(vec![v]));
                }
            },
        }
    } else {
        if dest_map.contains_key(subkey) {
            bail!("unexpected override existing field under {subkey:?}");
        }
        dest_map.insert(subkey.to_string(), value.unwrap_or(Value::Null));
    }
    Ok(())
}

fn add_params_map<'a>(
    m: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    if !m.contains_key(key) {
        m.insert(key.to_string(), Value::Object(Map::new()));
    }
    match m.get_mut(key).unwrap() {
        Value::Object(map) => Ok(map),
        other => bail!(
            "expected map type under {key:?}, got {}",
            go_type_name(other)
        ),
    }
}

fn add_params_slice<'a>(
    m: &'a mut Map<String, Value>,
    prevkey: &str,
    newkey: &str,
) -> Result<&'a mut Map<String, Value>> {
    if !m.contains_key(prevkey) {
        m.insert(prevkey.to_string(), Value::Array(Vec::new()));
    }
    match m.get_mut(prevkey).unwrap() {
        Value::Array(arr) => {
            let reuse_last = match arr.last() {
                Some(Value::Object(last)) => match last.get(newkey) {
                    None | Some(Value::Array(_)) => true,
                    Some(_) => false,
                },
                _ => false,
            };
            if !reuse_last {
                arr.push(Value::Object(Map::new()));
            }
            match arr.last_mut().unwrap() {
                Value::Object(map) => Ok(map),
                _ => unreachable!(),
            }
        }
        other => bail!(
            "expected array type under {prevkey:?}, got {}",
            go_type_name(other)
        ),
    }
}

fn magic_field_value(v: &str, stdin: &mut dyn Read) -> Result<Value> {
    if let Some(path) = v.strip_prefix('@') {
        return Ok(Value::String(read_user_file(path, stdin)?));
    }

    if let Ok(n) = v.parse::<i64>() {
        return Ok(Value::Number(n.into()));
    }

    Ok(match v {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => Value::String(v.to_string()),
    })
}

fn read_user_file(path: &str, stdin: &mut dyn Read) -> Result<String> {
    let bytes = if path == "-" {
        let mut buf = Vec::new();
        stdin
            .read_to_end(&mut buf)
            .map_err(|e| anyhow::anyhow!("open -: {e}"))?;
        buf
    } else {
        std::fs::read(path).map_err(|e| anyhow::anyhow!("open {path}: {e}"))?
    };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn go_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "<nil>",
        Value::Bool(_) => "bool",
        Value::Number(_) => "int",
        Value::String(_) => "string",
        Value::Array(_) => "[]interface {}",
        Value::Object(_) => "map[string]interface {}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_fields_basic() {
        let mut stdin: &[u8] = b"pasted contents";
        let params = parse_fields(
            &strs(&[
                "robot=Hubot",
                "destroyer=false",
                "helper=true",
                "location=@work",
            ]),
            &strs(&["input=@-", "enabled=true", "victories=123"]),
            &mut stdin,
        )
        .unwrap();

        let expect = serde_json::json!({
            "robot": "Hubot",
            "destroyer": "false",
            "helper": "true",
            "location": "@work",
            "input": "pasted contents",
            "enabled": true,
            "victories": 123,
        });
        assert_eq!(Value::Object(params), expect);
    }

    #[test]
    fn parse_fields_nested() {
        let mut stdin: &[u8] = b"pasted contents";
        let params = parse_fields(
            &strs(&[
                "branch[name]=patch-1",
                "robots[]=Hubot",
                "robots[]=Dependabot",
                "labels[][name]=bug",
                "labels[][color]=red",
                "labels[][colorOptions][]=red",
                "labels[][colorOptions][]=blue",
                "labels[][name]=feature",
                "labels[][color]=green",
                "labels[][colorOptions][]=red",
                "labels[][colorOptions][]=green",
                "labels[][colorOptions][]=yellow",
                "nested[][key1][key2][key3]=value",
                "empty[]",
            ]),
            &strs(&["branch[protections]=true", "ids[]=123", "ids[]=456"]),
            &mut stdin,
        )
        .unwrap();

        let expected = r#"{
  "branch": {
    "name": "patch-1",
    "protections": true
  },
  "robots": [
    "Hubot",
    "Dependabot"
  ],
  "labels": [
    {
      "name": "bug",
      "color": "red",
      "colorOptions": [
        "red",
        "blue"
      ]
    },
    {
      "name": "feature",
      "color": "green",
      "colorOptions": [
        "red",
        "green",
        "yellow"
      ]
    }
  ],
  "nested": [
    {
      "key1": {
        "key2": {
          "key3": "value"
        }
      }
    }
  ],
  "empty": [],
  "ids": [
    123,
    456
  ]
}"#;
        assert_eq!(
            serde_json::to_string_pretty(&Value::Object(params)).unwrap(),
            expected
        );
    }

    #[test]
    fn parse_fields_errors() {
        let cases: &[(&[&str], &str)] = &[
            (
                &["object[field]=A", "object[field][]=this should be an error"],
                r#"expected array type under "field", got string"#,
            ),
            (
                &[
                    "object[field]=B",
                    "object[field][field2]=this should be an error",
                ],
                r#"expected map type under "field", got string"#,
            ),
            (
                &[
                    "object[field][field2]=C",
                    "object[field]=this should be an error",
                ],
                r#"unexpected override existing field under "field""#,
            ),
            (
                &[
                    "object[field][field2]=D",
                    "object[field][]=this should be an error",
                ],
                r#"expected array type under "field", got map[string]interface {}"#,
            ),
            (
                &["object[field][]=E", "object[field]=this should be an error"],
                r#"unexpected override existing field under "field""#,
            ),
            (
                &[
                    "object[field][]=F",
                    "object[field][field2]=this should be an error",
                ],
                r#"expected map type under "field", got []interface {}"#,
            ),
        ];

        for (fields, expected) in cases {
            let mut stdin: &[u8] = b"";
            let err = parse_fields(&strs(fields), &[], &mut stdin).unwrap_err();
            assert_eq!(err.to_string(), *expected);
        }
    }

    #[test]
    fn magic_field_value_cases() {
        let mut stdin: &[u8] = b"";

        assert_eq!(
            magic_field_value("hello", &mut stdin).unwrap(),
            Value::String("hello".into())
        );
        assert_eq!(
            magic_field_value("true", &mut stdin).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            magic_field_value("false", &mut stdin).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(magic_field_value("null", &mut stdin).unwrap(), Value::Null);

        assert_eq!(
            magic_field_value("123", &mut stdin).unwrap(),
            Value::Number(123.into())
        );
        assert_eq!(
            magic_field_value("-45", &mut stdin).unwrap(),
            Value::Number((-45).into())
        );
        assert_eq!(
            magic_field_value("1.5", &mut stdin).unwrap(),
            Value::String("1.5".into())
        );
        assert_eq!(
            magic_field_value("99999999999999999999", &mut stdin).unwrap(),
            Value::String("99999999999999999999".into())
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gh-test");
        std::fs::write(&path, "file contents").unwrap();
        assert_eq!(
            magic_field_value(&format!("@{}", path.display()), &mut stdin).unwrap(),
            Value::String("file contents".into())
        );

        assert!(magic_field_value("@", &mut stdin).is_err());
    }
}
