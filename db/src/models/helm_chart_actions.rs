use crate::{DbError, DbResult, DbTable, UserDeploymentRole};
use platz_ui_schema::{UiSchema, UiSchemaInputError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum HelmChartActionEndpoint {
    #[serde(rename = "standard_ingress")]
    StandardIngress,
}

#[derive(Debug, Deserialize)]
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
    pub async fn generate_body(
        &self,
        inputs: serde_json::Value,
    ) -> Result<serde_json::Value, UiSchemaInputError<DbTable>> {
        let ui_schema = match self.ui_schema.as_ref() {
            None => return Ok(inputs),
            Some(ui_schema) => ui_schema,
        };
        Ok(ui_schema.get_values::<DbTable>(&inputs).await?.into())
    }
}

#[derive(Debug, Deserialize)]
pub struct HelmChartActionsSchema {
    pub schema_version: u64,
    pub actions: Vec<HelmChartActionSchema>,
}

impl HelmChartActionsSchema {
    pub fn find(&self, action_id: &str) -> DbResult<&HelmChartActionSchema> {
        match self.actions.iter().find(|action| action.id == action_id) {
            Some(action_schema) => Ok(action_schema),
            None => Err(DbError::HelmChartNoSuchAction(action_id.to_owned())),
        }
    }
}
