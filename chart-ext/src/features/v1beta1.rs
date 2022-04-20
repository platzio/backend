use super::v0;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExtFeaturesV1Beta1 {
    pub api_version: crate::versions::V1Beta1,
    pub kind: crate::versions::Features,
    pub spec: v0::ChartExtFeatures,
}
