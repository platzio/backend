use maplit::hashmap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct JsonDiffPair(Value, Value);

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct JsonDiff(HashMap<String, JsonDiffPair>);

pub fn json_diff(old: &Value, new: &Value) -> JsonDiff {
    if old == new {
        return Default::default();
    }
    match old {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            JsonDiff(hashmap!("".to_owned() => JsonDiffPair(old.clone(), new.clone())))
        }
        Value::Object(old_obj) => match new {
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => {
                JsonDiff(hashmap!("".to_owned() => JsonDiffPair(old.clone(), new.clone())))
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
                            diffs.0.insert(
                                old_key.to_owned(),
                                JsonDiffPair(old_value.clone(), json!(null)),
                            );
                        }
                    }
                }

                for (new_key, new_value) in new_obj {
                    if !old_obj.contains_key(new_key) {
                        diffs.0.insert(
                            new_key.to_owned(),
                            JsonDiffPair(json!(null), new_value.clone()),
                        );
                    }
                }

                diffs
            }
        },
    }
}

fn hashmap_merge_with_prefix(diffs: &mut JsonDiff, changes: JsonDiff, prefix: &str) {
    for (k, v) in changes.0.into_iter() {
        diffs.0.insert(
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
        assert_eq!(json_diff(&json!(1), &json!(1)), JsonDiff::default());
    }

    #[test]
    fn diff_same_null() {
        assert_eq!(json_diff(&json!(null), &json!(null)), JsonDiff::default());
    }

    #[test]
    fn diff_different_primitives() {
        assert_eq!(
            json_diff(&json!(1), &json!(2)),
            JsonDiff(hashmap! ("".to_owned() => JsonDiffPair(json!(1), json!(2))))
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
            JsonDiff(hashmap! (
                "a".to_owned() => JsonDiffPair(json!(1), json!(2)),
                "b".to_owned() => JsonDiffPair(json!(2), json!(null)),
                "c".to_owned() => JsonDiffPair(json!(null), json!(3)),
            )),
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
            JsonDiff(hashmap! (
                "a".to_owned() => JsonDiffPair(json!(1), json!(2)),
                "b".to_owned() => JsonDiffPair(json!(2), json!(null)),
                "c".to_owned() => JsonDiffPair(
                    json!({
                    "x": "xxx",
                    "y": "yyy",
                    }),
                    json!(3)
                ),
            )),
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
            JsonDiff(hashmap! (
                "config.runtime.beta_force".to_owned() => JsonDiffPair(json!(2), json!(3)),
            )),
        );
    }
}
