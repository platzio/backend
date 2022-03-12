use platz_chart_ext::{UiSchemaCollections, UiSchemaInputError};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, strum::Display)]
pub enum TestDb {
    First,
    Second,
    Third,
}

#[derive(Debug, thiserror::Error)]
pub enum TestDbError {}

#[async_trait::async_trait]
impl UiSchemaCollections for TestDb {
    type Error = TestDbError;

    async fn resolve(
        &self,
        _env_id: Uuid,
        id: &str,
        property: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<Self::Error>> {
        let id = i64::from_str(id)
            .map_err(|_| UiSchemaInputError::InvalidCollectionId(id.to_owned()))?;

        match self {
            Self::First => match property {
                "id" => Ok(id.to_string().into()),
                "a" => Ok(format!("a{}", id).into()),
                _ => Err(UiSchemaInputError::UnknownProperty(
                    property.to_owned(),
                    self.to_string(),
                )),
            },
            Self::Second => match property {
                "id" => Ok(id.to_string().into()),
                "b" => Ok(format!("b{}", id).into()),
                _ => Err(UiSchemaInputError::UnknownProperty(
                    property.to_owned(),
                    self.to_string(),
                )),
            },
            _ => Err(UiSchemaInputError::UnsupportedCollection(self.to_string())),
        }
    }
}
