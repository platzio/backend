use crate::DbError;
use platz_chart_ext::{UiSchemaCollections, UiSchemaInputError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DbTable {
    DeploymentKinds,
    DeploymentResources,
    DeploymentResourceTypes,
    Deployments,
    DeploymentTasks,
    DeploymentPermissions,
    Envs,
    EnvUserPermissions,
    HelmRegistries,
    HelmCharts,
    HelmTagFormats,
    K8sClusters,
    K8sResources,
    Secrets,
    Users,
}

#[async_trait::async_trait]
impl UiSchemaCollections for DbTable {
    type Error = DbError;

    async fn resolve(
        &self,
        env_id: Uuid,
        id: &str,
        property: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<Self::Error>> {
        let id = Uuid::parse_str(id)
            .map_err(|_| UiSchemaInputError::InvalidCollectionId(id.to_owned()))?;
        match self {
            Self::Deployments => {
                // TODO: Return Option from Deployment::find and convert to UiSchemaInputError::CollectionItemNotFound when None
                let deployment = crate::schema::Deployment::find(id).await?;
                let kind_obj = crate::schema::DeploymentKind::find(deployment.kind_id).await?;
                // TODO: Check deployment is in env_id
                match property {
                    "id" => Ok(deployment.id.to_string().into()),
                    "created_at" => Ok(deployment.created_at.to_string().into()),
                    "name" => Ok(deployment.name.into()),
                    "kind" => Ok(kind_obj.name.into()),
                    "cluster_id" => Ok(deployment.cluster_id.to_string().into()),
                    "enabled" => Ok(deployment.enabled.into()),
                    _ => Err(UiSchemaInputError::UnknownProperty(
                        property.to_owned(),
                        self.to_string(),
                    )),
                }
            }
            Self::Secrets => {
                // TODO: Return Option from Secret::find and convert to UiSchemaInputError::CollectionItemNotFound when None
                let secret = crate::schema::Secret::find(id).await?;
                if secret.env_id == env_id {
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
                } else {
                    Err(UiSchemaInputError::CollectionItemNotFound(
                        self.to_string(),
                        id.to_string(),
                    ))
                }
            }
            _ => Err(UiSchemaInputError::UnsupportedCollection(self.to_string())),
        }
    }
}

impl std::fmt::Display for DbTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_value(self)
                .expect("Could not serialize DbTable as Value")
                .as_str()
                .expect("Could not serialize DbTable as string")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::DbTable;

    #[test]
    fn test_db_table_to_string() {
        let table = DbTable::DeploymentPermissions;
        assert_eq!(table.to_string(), "deployment_permissions");
    }
}
