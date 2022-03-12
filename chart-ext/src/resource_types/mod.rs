pub mod v1;

use crate::serde_utils::one_or_many;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChartExtResourceTypes {
    #[serde(deserialize_with = "one_or_many")]
    pub inner: Vec<ChartExtResourceType>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtResourceType {
    V1(v1::ChartExtResourceType),
}
