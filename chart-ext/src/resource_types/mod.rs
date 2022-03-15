pub mod v1;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChartExtResourceTypes {
    pub inner: Vec<ChartExtResourceType>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtResourceType {
    V1(v1::ChartExtResourceType),
}
