use crate::collection::UiSchemaCollections;
use crate::error::UiSchemaInputError;
use crate::values_ui::UiSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChartExtActions {
    pub schema_version: u64,
    pub actions: Vec<ChartExtAction>,
}

impl ChartExtActions {
    pub fn find(&self, action_id: &str) -> Option<&ChartExtAction> {
        self.actions.iter().find(|action| action.id == action_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserDeploymentRole {
    Owner,
    Maintainer,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ChartExtActionEndpoint {
    #[serde(rename = "standard_ingress")]
    StandardIngress,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChartExtAction {
    pub id: String,
    pub allowed_role: UserDeploymentRole,
    pub endpoint: ChartExtActionEndpoint,
    pub path: String,
    pub method: String,
    pub title: String,
    pub fontawesome_icon: Option<String>,
    pub description: String,
    pub ui_schema: Option<UiSchema>,
}

impl ChartExtAction {
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
