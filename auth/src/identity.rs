use platz_db::Identity;
use serde::Serialize;
use std::borrow::Borrow;

#[derive(Serialize)]
pub struct ApiIdentity(Identity);

impl ApiIdentity {
    pub fn inner(&self) -> &Identity {
        &self.0
    }

    pub fn into_inner(self) -> Identity {
        self.0
    }
}

impl From<Identity> for ApiIdentity {
    fn from(identity: Identity) -> Self {
        Self(identity)
    }
}

impl From<ApiIdentity> for Identity {
    fn from(api_identity: ApiIdentity) -> Self {
        api_identity.into_inner()
    }
}

impl Borrow<Identity> for ApiIdentity {
    fn borrow(&self) -> &Identity {
        self.inner()
    }
}
