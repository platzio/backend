use crate::actions::v0::{ChartExtActionTarget, UserDeploymentRole};
use crate::values_ui::UiSchemaV0;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExtResourceType {
    pub api_version: crate::versions::V1Beta1,
    pub kind: crate::versions::ResourceType,
    pub key: String,
    pub spec: ChartExtResourceTypeSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChartExtResourceTypeSpec {
    pub name_singular: String,
    pub name_plural: String,
    pub fontawesome_icon: String,
    #[serde(default)]
    pub global: bool,
    pub values_ui: UiSchemaV0,
    #[serde(default)]
    pub lifecycle: ChartExtResourceLifecycle,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChartExtResourceLifecycle {
    pub create: Option<ResourceLifecycle>,
    pub update: Option<ResourceLifecycle>,
    pub delete: Option<ResourceLifecycle>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceLifecycle {
    pub allowed_role: Option<UserDeploymentRole>,
    pub target: Option<ChartExtActionTarget>,
}
