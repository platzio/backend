use crate::{DbError, DbTable, DeploymentResource, DeploymentResourceType};
use platz_chart_ext::{UiSchemaCollections, UiSchemaInputError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DbTableOrDeploymentResource {
    DeploymentResourceType { deployment: String, r#type: String },
    CollectionName(String),
}

impl From<DbTable> for DbTableOrDeploymentResource {
    fn from(db_table: DbTable) -> Self {
        Self::CollectionName(
            serde_json::to_value(db_table)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned(),
        )
    }
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
            Self::DeploymentResourceType { deployment, r#type } => {
                DeploymentResourceType::find_by_kind_and_key(env_id, deployment, r#type)
                    .await
                    .map_err(UiSchemaInputError::CollectionError)?
            }
            Self::CollectionName(name) => match serde_json::from_str::<DbTable>(name) {
                Ok(db_table) => {
                    return db_table.resolve(env_id, id, property).await;
                }
                Err(_) => DeploymentResourceType::find_all_by_key(env_id, name)
                    .await
                    .map_err(UiSchemaInputError::CollectionError)?,
            },
        };
        let id = Uuid::parse_str(id)
            .map_err(|_| UiSchemaInputError::InvalidCollectionId(id.to_owned()))?;
        let resource = DeploymentResource::find_of_type(resource_type.id, id)
            .await
            .map_err(UiSchemaInputError::CollectionError)?;
        resource
            .props
            .get(property)
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                UiSchemaInputError::CollectionItemNotFound(self.to_string(), id.to_string())
            })
    }
}

impl std::fmt::Display for DbTableOrDeploymentResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeploymentResourceType { deployment, r#type } => {
                write!(f, "{}/{}", deployment, r#type)
            }
            Self::CollectionName(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DbTableOrDeploymentResource;
    use serde_json::{from_value, json, to_value};

    #[test]
    fn test() {
        assert_eq!(
            from_value::<DbTableOrDeploymentResource>(json!("deployments")).unwrap(),
            DbTableOrDeploymentResource::CollectionName("deployments".to_owned())
        );

        assert_eq!(
            to_value(DbTableOrDeploymentResource::CollectionName(
                "users".to_owned()
            ))
            .unwrap(),
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
