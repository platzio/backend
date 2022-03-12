pub mod v0;
pub mod v1;

use serde::{Deserialize, Serialize};
pub use v0::{ChartExtActionEndpoint, ChartExtActionTarget, ChartExtActionTargetResolver};

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtActions {
    V1(v1::ChartExtActions),
    V0(v0::ChartExtActions),
}

impl ChartExtActions {
    pub fn find(&self, action_id: &str) -> Option<&v0::ChartExtAction> {
        match self {
            Self::V1(v1) => v1.find(action_id),
            Self::V0(v0) => v0.find(action_id),
        }
    }
}
