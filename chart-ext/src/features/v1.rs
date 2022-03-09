use super::v0;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExtFeatures {
    pub api_version: crate::versions::V1,
    pub kind: crate::versions::Features,
    pub spec: v0::ChartExtFeatures,
}
