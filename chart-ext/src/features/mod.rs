pub mod v0;
pub mod v1;

use serde::{Deserialize, Serialize};
pub use v0::ChartExtCardinality;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ChartExtFeatures {
    V1(v1::ChartExtFeatures),
    V0(v0::ChartExtFeatures),
}

impl Default for ChartExtFeatures {
    fn default() -> Self {
        Self::V1(Default::default())
    }
}

impl ChartExtFeatures {
    pub fn standard_ingress(&self) -> bool {
        match self {
            Self::V1(v1) => v1.spec.standard_ingress,
            Self::V0(v0) => v0.standard_ingress,
        }
    }

    pub fn status(&self) -> Option<&v0::ChartExtStatusFeature> {
        match self {
            Self::V1(v1) => v1.spec.status.as_ref(),
            Self::V0(v0) => v0.status.as_ref(),
        }
    }

    pub fn cardinality(&self) -> &v0::ChartExtCardinality {
        match self {
            Self::V1(v1) => &v1.spec.cardinality,
            Self::V0(v0) => &v0.cardinality,
        }
    }

    pub fn reinstall_dependencies(&self) -> bool {
        match self {
            Self::V1(v1) => v1.spec.reinstall_dependencies,
            Self::V0(v0) => v0.reinstall_dependencies,
        }
    }

    pub fn node_selector_paths(&self) -> &Vec<Vec<String>> {
        match self {
            Self::V1(v1) => &v1.spec.node_selector_paths,
            Self::V0(v0) => &v0.node_selector_paths,
        }
    }

    pub fn tolerations_paths(&self) -> &Vec<Vec<String>> {
        match self {
            Self::V1(v1) => &v1.spec.tolerations_paths,
            Self::V0(v0) => &v0.tolerations_paths,
        }
    }
}
