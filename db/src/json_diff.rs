use maplit::hashmap;
use serde_json::{json, Value};
use std::collections::HashMap;

pub type JsonDiff = HashMap<String, (Value, Value)>;

pub fn json_diff(old: &Value, new: &Value) -> JsonDiff {
    if old == new {
        return Default::default();
    }
    match old {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            hashmap!("".to_owned() => (old.clone(), new.clone()))
        }
        Value::Object(old_obj) => match new {
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => {
                hashmap!("".to_owned() => (old.clone(), new.clone()))
            }
            Value::Object(new_obj) => {
                let mut diffs = Default::default();

                for (old_key, old_value) in old_obj {
                    match new_obj.get(old_key) {
                        Some(new_value) => {
                            let inner = json_diff(old_value, new_value);
                            hashmap_merge_with_prefix(&mut diffs, inner, old_key);
                        }
                        None => {
                            diffs.insert(old_key.to_owned(), (old_value.clone(), json!(null)));
                        }
                    }
                }

                for (new_key, new_value) in new_obj {
                    if !old_obj.contains_key(new_key) {
                        diffs.insert(new_key.to_owned(), (json!(null), new_value.clone()));
                    }
                }

                diffs
            }
        },
    }
}

fn hashmap_merge_with_prefix(diffs: &mut JsonDiff, changes: JsonDiff, prefix: &str) {
    for (k, v) in changes.into_iter() {
        diffs.insert(
            vec![prefix, &k]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
                .join("."),
            v,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_same_primitive() {
        assert_eq!(json_diff(&json!(1), &json!(1)), hashmap! {});
    }

    #[test]
    fn diff_same_null() {
        assert_eq!(json_diff(&json!(null), &json!(null)), hashmap! {});
    }

    #[test]
    fn diff_different_primitives() {
        assert_eq!(
            json_diff(&json!(1), &json!(2)),
            hashmap! ("".to_owned() => (json!(1), json!(2)))
        );
    }

    #[test]
    fn diff_different_objects1() {
        assert_eq!(
            json_diff(
                &json!({
                    "a": 1,
                    "b": 2,
                }),
                &json!({
                    "a": 2,
                    "c": 3,
                })
            ),
            hashmap! (
                "a".to_owned() => (json!(1), json!(2)),
                "b".to_owned() => (json!(2), json!(null)),
                "c".to_owned() => (json!(null), json!(3)),
            ),
        );
    }

    #[test]
    fn diff_different_objects2() {
        assert_eq!(
            json_diff(
                &json!({
                    "a": 1,
                    "b": 2,
                    "c": {
                        "x": "xxx",
                        "y": "yyy",
                    }
                }),
                &json!({
                    "a": 2,
                    "c": 3,
                })
            ),
            hashmap! (
                "a".to_owned() => (json!(1), json!(2)),
                "b".to_owned() => (json!(2), json!(null)),
                "c".to_owned() => (
                    json!({
                    "x": "xxx",
                    "y": "yyy",
                    }),
                    json!(3)
                ),
            ),
        );
    }

    #[test]
    fn diff_different_objects3() {
        assert_eq!(
            json_diff(
                &json!({
                    "config": {
                        "runtime": {
                            "alpha_force": 1,
                            "beta_force": 2,
                        }
                    }
                }),
                &json!({
                    "config": {
                        "runtime": {
                            "alpha_force": 1,
                            "beta_force": 3,
                        }
                    }
                })
            ),
            hashmap! (
                "config.runtime.beta_force".to_owned() => (json!(2), json!(3)),
            ),
        );
    }
}
