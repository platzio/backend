use crate::UiSchemaInputError;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[async_trait]
pub trait UiSchemaCollections
where
    Self: DeserializeOwned + Serialize,
{
    type Error: std::fmt::Display;

    async fn resolve(
        &self,
        id: &str,
        property: &str,
    ) -> Result<serde_json::Value, UiSchemaInputError<Self>>;
}
