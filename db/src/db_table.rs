use crate::DbError;
use platz_chart_ext::{UiSchemaCollections, UiSchemaInputError};
use serde::{Deserialize, Serialize};
use strum::Display;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum DbTable {
    #[serde(rename = "k8s_clusters")]
    K8sClusters,
    #[serde(rename = "k8s_resources")]
    K8sResources,
    #[serde(rename = "helm_registries")]
    HelmRegistries,
    #[serde(rename = "helm_charts")]
    HelmCharts,
    #[serde(rename = "deployment_configs")]
    DeploymentConfigs,
    #[serde(rename = "deployments")]
    Deployments,
    #[serde(rename = "deployment_tasks")]
    DeploymentTasks,
    #[serde(rename = "deployment_permissions")]
    DeploymentPermissions,
    #[serde(rename = "secrets")]
    Secrets,
    #[serde(rename = "envs")]
    Envs,
    #[serde(rename = "env_user_permissions")]
    EnvUserPermissions,
    #[serde(rename = "users")]
    Users,
}

#[async_trait::async_trait]
impl UiSchemaCollections for DbTable {
    type Error = DbError;

    async fn resolve(
        &self,
        id: &str,
        property: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<Self>> {
        let id = Uuid::parse_str(id)
            .map_err(|_| UiSchemaInputError::InvalidCollectionId(id.to_owned()))?;
        match self {
            Self::Deployments => {
                let deployment = crate::models::Deployment::find(id)
                    .await
                    .map_err(UiSchemaInputError::CollectionError)?;
                match property {
                    "id" => Ok(deployment.id.to_string().into()),
                    "created_at" => Ok(deployment.created_at.to_string().into()),
                    "name" => Ok(deployment.name.into()),
                    "kind" => Ok(deployment.kind.into()),
                    "cluster_id" => Ok(deployment.cluster_id.to_string().into()),
                    "enabled" => Ok(deployment.enabled.into()),
                    _ => Err(UiSchemaInputError::UnknownProperty(
                        property.to_owned(),
                        self.to_string(),
                    )),
                }
            }
            Self::Secrets => {
                let secret = crate::models::Secret::find(id)
                    .await
                    .map_err(UiSchemaInputError::CollectionError)?;
                match property {
                    "id" => Ok(secret.id.to_string().into()),
                    "created_at" => Ok(secret.created_at.to_string().into()),
                    "updated_at" => Ok(secret.updated_at.to_string().into()),
                    "env_id" => Ok(secret.env_id.to_string().into()),
                    "collection" => Ok(secret.collection.into()),
                    "name" => Ok(secret.name.into()),
                    "contents" => Ok(secret.contents.into()),
                    _ => Err(UiSchemaInputError::UnknownProperty(
                        property.to_owned(),
                        self.to_string(),
                    )),
                }
            }
            _ => Err(UiSchemaInputError::UnsupportedCollection(self.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DbTable;

    #[test]
    fn test_db_table_serialize() {
        let table = DbTable::DeploymentPermissions;
        assert_eq!(
            serde_json::to_value(&table).unwrap().as_str().unwrap(),
            "deployment_permissions"
        );
    }
}
