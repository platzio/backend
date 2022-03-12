use super::{AccessToken, AuthError};
use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::{err, ok, BoxFuture, FutureExt, TryFutureExt};
use platz_db::User;
use serde::Serialize;

#[derive(Serialize)]
pub struct CurUser {
    user: User,
}

impl CurUser {
    fn new(user: User) -> Self {
        Self { user }
    }

    pub fn user(&self) -> &User {
        &self.user
    }
}

impl FromRequest for CurUser {
    type Error = AuthError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        AccessToken::from_request(req, payload)
            .and_then(|token| User::find(token.user_id()).map_err(AuthError::DatabaseError))
            .and_then(|option| match option {
                None => err(AuthError::UserNotFound),
                Some(user) => ok(user),
            })
            .and_then(|user| ok(Self::new(user)))
            .boxed()
    }
}
