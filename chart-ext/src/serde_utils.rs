// https://github.com/serde-rs/serde/issues/1907#issuecomment-708989249

use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum OneOrMany<T> {
    One(T),
    Vec(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(from: OneOrMany<T>) -> Self {
        match from {
            OneOrMany::One(val) => vec![val],
            OneOrMany::Vec(vec) => vec,
        }
    }
}

pub(crate) fn one_or_many<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    let e: OneOrMany<T> = Deserialize::deserialize(deserializer)?;
    Ok(e.into())
}

#[cfg(test)]
mod tests {
    use super::one_or_many;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Item {
        a: String,
        b: u64,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    #[serde(transparent)]
    struct Test {
        #[serde(deserialize_with = "one_or_many")]
        arr: Vec<Item>,
    }

    #[test]
    fn test1() {
        let value = json!({
            "a": "hi",
            "b": 3 as u64,
        });

        let test: Test = serde_json::from_value(value).unwrap();

        assert_eq!(
            test,
            Test {
                arr: vec![Item {
                    a: "hi".to_owned(),
                    b: 3
                }]
            }
        );
    }

    #[test]
    fn test2() {
        let value = json!([
            {
                "a": "hi",
                "b": 3 as u64,
            },
            {
                "a": "hello",
                "b": 9 as u64,
            }
        ]);

        let test: Test = serde_json::from_value(value).unwrap();

        assert_eq!(
            test,
            Test {
                arr: vec![
                    Item {
                        a: "hi".to_owned(),
                        b: 3,
                    },
                    Item {
                        a: "hello".to_owned(),
                        b: 9,
                    },
                ]
            }
        );
    }
}
