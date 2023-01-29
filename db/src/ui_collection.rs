use crate::{DbError, DbTable, DeploymentResource, DeploymentResourceType};
use platz_chart_ext::{UiSchemaCollections, UiSchemaInputError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DbTableOrDeploymentResource {
    DbTable(DbTable),
    DeploymentResourceType { deployment: String, r#type: String },
    LegacyCollectionName(String),
}

#[async_trait::async_trait]
impl UiSchemaCollections for DbTableOrDeploymentResource {
    type Error = DbError;

    async fn resolve(
        &self,
        env_id: Uuid,
        id: &str,
        property: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<Self::Error>> {
        let resource_type = match self {
            Self::DbTable(db_table) => return db_table.resolve(env_id, id, property).await,
            Self::DeploymentResourceType { deployment, r#type } => {
                DeploymentResourceType::find_by_env_kind_and_key(
                    env_id,
                    deployment.to_owned(),
                    r#type.to_owned(),
                )
                .await
                .map_err(UiSchemaInputError::CollectionError)?
            }
            Self::LegacyCollectionName(name) => {
                DeploymentResourceType::find_all_by_key(env_id, name.to_owned())
                    .await
                    .map_err(UiSchemaInputError::CollectionError)?
            }
        };
        let id = Uuid::parse_str(id)
            .map_err(|_| UiSchemaInputError::InvalidCollectionId(id.to_owned()))?;
        let resource = DeploymentResource::find_of_type(resource_type.id, id)
            .await
            .map_err(UiSchemaInputError::CollectionError)?;
        match property {
            "id" => Ok(id.to_string().into()),
            "name" => Ok(resource.name.into()),
            _ => resource
                .props
                .get(property)
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    UiSchemaInputError::UnknownProperty(property.to_owned(), self.to_string())
                }),
        }
    }
}

impl std::fmt::Display for DbTableOrDeploymentResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbTable(db_table) => db_table.fmt(f),
            Self::DeploymentResourceType { deployment, r#type } => {
                write!(f, "{deployment}/{type}")
            }
            Self::LegacyCollectionName(name) => write!(f, "{name}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DbTableOrDeploymentResource;
    use crate::DbTable;
    use serde_json::{from_value, json, to_value};

    #[test]
    fn test() {
        assert_eq!(
            from_value::<DbTableOrDeploymentResource>(json!("deployments")).unwrap(),
            DbTableOrDeploymentResource::DbTable(DbTable::Deployments)
        );

        assert_eq!(
            to_value(DbTableOrDeploymentResource::DbTable(DbTable::Users)).unwrap(),
            json!("users")
        );

        assert_eq!(
            from_value::<DbTableOrDeploymentResource>(json!({
                "deployment": "ShopManager",
                "type": "shop",
            }))
            .unwrap(),
            DbTableOrDeploymentResource::DeploymentResourceType {
                deployment: "ShopManager".to_owned(),
                r#type: "shop".to_owned(),
            }
        );
    }
}
