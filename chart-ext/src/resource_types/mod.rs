pub mod v1beta1;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChartExtResourceTypes {
    pub inner: Vec<ChartExtResourceType>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtResourceType {
    V1Beta1(v1beta1::ChartExtResourceType),
}
