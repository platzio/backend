use super::collection::UiSchemaCollections;
use super::error::UiSchemaInputError;
use super::values_ui::UiSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "apiVersion")]
pub enum HelmChartActionsSchema {
    #[serde(rename = "platz.io/actions/v1")]
    V1(HelmChartActionsV1),
}

impl HelmChartActionsSchema {
    pub fn find(&self, action_id: &str) -> Option<&HelmChartActionSchema> {
        match self {
            Self::V1(v1) => v1.find(action_id),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelmChartActionsV1 {
    pub actions: Vec<HelmChartActionSchema>,
}

impl HelmChartActionsV1 {
    pub fn find(&self, action_id: &str) -> Option<&HelmChartActionSchema> {
        self.actions.iter().find(|action| action.id == action_id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum HelmChartActionEndpoint {
    #[serde(rename = "standard_ingress")]
    StandardIngress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserDeploymentRole {
    Owner,
    Maintainer,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HelmChartActionSchema {
    pub id: String,
    pub allowed_role: UserDeploymentRole,
    pub endpoint: HelmChartActionEndpoint,
    pub path: String,
    pub method: String,
    pub title: String,
    pub fontawesome_icon: Option<String>,
    pub description: String,
    pub ui_schema: Option<UiSchema>,
}

impl HelmChartActionSchema {
    pub async fn generate_body<C>(
        &self,
        inputs: serde_json::Value,
    ) -> Result<serde_json::Value, UiSchemaInputError<C>>
    where
        C: UiSchemaCollections,
    {
        let ui_schema = match self.ui_schema.as_ref() {
            None => return Ok(inputs),
            Some(ui_schema) => ui_schema,
        };
        Ok(ui_schema.get_values::<C>(&inputs).await?.into())
    }
}
