pub mod v0;
pub mod v1beta1;

use serde::{Deserialize, Serialize};
pub use v0::ChartExtCardinality;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtFeatures {
    V1Beta1(v1beta1::ChartExtFeaturesV1Beta1),
    V0(v0::ChartExtFeatures),
}

impl Default for ChartExtFeatures {
    fn default() -> Self {
        Self::V1Beta1(Default::default())
    }
}

impl ChartExtFeatures {
    pub fn standard_ingress(&self) -> bool {
        match self {
            Self::V1Beta1(features) => features.spec.standard_ingress,
            Self::V0(features) => features.standard_ingress,
        }
    }

    pub fn status(&self) -> Option<&v0::ChartExtStatusFeature> {
        match self {
            Self::V1Beta1(features) => features.spec.status.as_ref(),
            Self::V0(features) => features.status.as_ref(),
        }
    }

    pub fn cardinality(&self) -> &v0::ChartExtCardinality {
        match self {
            Self::V1Beta1(features) => &features.spec.cardinality,
            Self::V0(features) => &features.cardinality,
        }
    }

    pub fn reinstall_dependencies(&self) -> bool {
        match self {
            Self::V1Beta1(features) => features.spec.reinstall_dependencies,
            Self::V0(features) => features.reinstall_dependencies,
        }
    }

    pub fn node_selector_paths(&self) -> &Vec<Vec<String>> {
        match self {
            Self::V1Beta1(features) => &features.spec.node_selector_paths,
            Self::V0(features) => &features.node_selector_paths,
        }
    }

    pub fn tolerations_paths(&self) -> &Vec<Vec<String>> {
        match self {
            Self::V1Beta1(features) => &features.spec.tolerations_paths,
            Self::V0(features) => &features.tolerations_paths,
        }
    }
}
