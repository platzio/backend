use crate::schema::{deployment::Deployment, user::User};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub enum Identity {
    User(Uuid),
    Bot(Uuid),
    Deployment(Uuid),
}

impl Identity {
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            Self::User(user_id) => Some(user_id.to_owned()),
            _ => None,
        }
    }

    pub fn deployment_id(&self) -> Option<Uuid> {
        match self {
            Self::Deployment(deployment_id) => Some(deployment_id.to_owned()),
            _ => None,
        }
    }
}

impl From<&User> for Identity {
    fn from(user: &User) -> Self {
        Self::User(user.id)
    }
}

impl From<&Deployment> for Identity {
    fn from(deployment: &Deployment) -> Self {
        Self::Deployment(deployment.id)
    }
}
