use crate::collection::UiSchemaCollections;
use crate::error::UiSchemaInputError;
use crate::values_ui::UiSchema;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChartExtActions {
    pub schema_version: u64,
    pub actions: Vec<ChartExtAction>,
}

impl ChartExtActions {
    pub fn find(&self, action_id: &str) -> Option<&ChartExtAction> {
        self.actions.iter().find(|action| action.id == action_id)
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserDeploymentRole {
    Owner,
    Maintainer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ChartExtActionEndpoint {
    #[serde(rename = "standard_ingress")]
    StandardIngress,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ChartExtActionMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChartExtActionTarget {
    pub endpoint: ChartExtActionEndpoint,
    pub path: String,
    pub method: ChartExtActionMethod,
}

impl ChartExtActionTarget {
    pub async fn call<R, T>(&self, resolver: &R, body: T) -> anyhow::Result<String>
    where
        R: ChartExtActionTargetResolver,
        T: Serialize,
    {
        let url = resolver.resolve(self).await?;
        let method = match self.method {
            ChartExtActionMethod::Get => reqwest::Method::GET,
            ChartExtActionMethod::Post => reqwest::Method::POST,
            ChartExtActionMethod::Put => reqwest::Method::PUT,
            ChartExtActionMethod::Patch => reqwest::Method::PATCH,
            ChartExtActionMethod::Delete => reqwest::Method::DELETE,
        };

        Ok(reqwest::Client::new()
            .request(method, url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }
}

#[async_trait::async_trait]
pub trait ChartExtActionTargetResolver {
    async fn resolve(&self, target: &ChartExtActionTarget) -> anyhow::Result<Url>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChartExtAction {
    pub id: String,
    pub allowed_role: UserDeploymentRole,
    #[serde(flatten)]
    pub target: ChartExtActionTarget,
    pub title: String,
    pub fontawesome_icon: Option<String>,
    pub description: String,
    pub ui_schema: Option<UiSchema>,
}

impl ChartExtAction {
    pub async fn generate_body<C>(
        &self,
        env_id: Uuid,
        inputs: serde_json::Value,
    ) -> Result<serde_json::Value, UiSchemaInputError<C::Error>>
    where
        C: UiSchemaCollections,
    {
        let ui_schema = match self.ui_schema.as_ref() {
            None => return Ok(inputs),
            Some(ui_schema) => ui_schema,
        };
        Ok(ui_schema.get_values::<C>(env_id, &inputs).await?.into())
    }
}
