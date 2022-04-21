use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum V1Beta1 {
    #[serde(rename = "platz.io/v1beta1")]
    Value,
}

impl Default for V1Beta1 {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum V1Beta2 {
    #[serde(rename = "platz.io/v1beta2")]
    Value,
}

impl Default for V1Beta2 {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ValuesUi {
    #[serde(rename = "ValuesUi")]
    Value,
}

impl Default for ValuesUi {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Action {
    #[serde(rename = "Action")]
    Value,
}

impl Default for Action {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Features {
    #[serde(rename = "Features")]
    Value,
}

impl Default for Features {
    fn default() -> Self {
        Self::Value
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ResourceType {
    #[serde(rename = "ResourceType")]
    Value,
}

impl Default for ResourceType {
    fn default() -> Self {
        Self::Value
    }
}
