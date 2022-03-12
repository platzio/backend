use super::v0;

use crate::serde_utils::one_or_many;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ChartExtActions {
    #[serde(deserialize_with = "one_or_many")]
    pub actions: Vec<ChartExtAction>,
}

impl ChartExtActions {
    pub fn find(&self, action_id: &str) -> Option<&v0::ChartExtAction> {
        self.actions
            .iter()
            .find(|action| action.spec.id == action_id)
            .map(|action| &action.spec)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExtAction {
    pub api_version: crate::versions::V1,
    pub kind: crate::versions::Action,
    pub spec: v0::ChartExtAction,
}
