use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum V1 {
    #[serde(rename = "platz.io/v1beta1")]
    Value,
}

impl Default for V1 {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ValuesUi {
    #[serde(rename = "ValuesUi")]
    Value,
}

impl Default for ValuesUi {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Action {
    #[serde(rename = "Action")]
    Value,
}

impl Default for Action {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Features {
    #[serde(rename = "Features")]
    Value,
}

impl Default for Features {
    fn default() -> Self {
        Self::Value
    }
}
