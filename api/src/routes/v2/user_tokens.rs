use crate::permissions::verify_site_admin;
use crate::result::{ApiError, ApiResult};
use actix_web::{web, HttpResponse};
use platz_auth::{generate_user_token, ApiIdentity, AuthError};
use platz_db::{DbError, NewUserToken, User, UserToken, UserTokenFilters};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

fn ensure_user_id(identity: &ApiIdentity) -> Result<Uuid, ApiError> {
    identity.inner().user_id().ok_or(ApiError::NoPermission)
}

async fn ensure_user(identity: &ApiIdentity) -> Result<User, ApiError> {
    User::find(ensure_user_id(identity)?).await?.ok_or_else(|| {
        ApiError::from(AuthError::BearerAuthenticationError(
            "Unknown user".to_owned(),
        ))
    })
}

async fn get_all(identity: ApiIdentity, filters: web::Query<UserTokenFilters>) -> ApiResult {
    let user = ensure_user(&identity).await?;
    let mut filters = filters.into_inner();
    if !user.is_admin {
        if filters.user_id.is_some() {
            return Ok(HttpResponse::Forbidden().json(json!({
                "message": "Non-admin users are not allowed to query for user-tokens of other users",
            })));
        }
        filters.user_id = Some(user.id);
    }
    Ok(HttpResponse::Ok().json(UserToken::all_filtered(filters).await?))
}

async fn get(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let user = ensure_user(&identity).await?;
    let user_token = UserToken::find(id.into_inner()).await?;
    if user.is_admin || (user.id == user_token.user_id) {
        Ok(HttpResponse::Ok().json(user_token))
    } else {
        Err(ApiError::from(DbError::NotFound))
    }
}

async fn get_token_user_id_and_verify_permissions(
    identity: &ApiIdentity,
    token_for_user_id: Option<Uuid>,
) -> Result<Uuid, ApiError> {
    let identity_user_id = ensure_user_id(identity)?;
    let token_user_id = token_for_user_id.unwrap_or(identity_user_id);
    if token_user_id != identity_user_id {
        // Only site admins can handle tokens for other users
        verify_site_admin(identity).await?;
    }
    Ok(token_user_id)
}

#[derive(Debug, Deserialize)]
pub struct ApiNewUserToken {
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct TokenCreationResponse {
    created_token: String,
}

async fn create(identity: ApiIdentity, new_user_token: web::Json<ApiNewUserToken>) -> ApiResult {
    let new_user_token = new_user_token.into_inner();
    let token_user_id =
        get_token_user_id_and_verify_permissions(&identity, new_user_token.user_id).await?;

    let user_token_info = generate_user_token().await?;
    NewUserToken {
        id: user_token_info.token_id,
        user_id: token_user_id,
        secret_hash: user_token_info.secret_hash,
    }
    .save()
    .await?;

    Ok(HttpResponse::Created().json(TokenCreationResponse {
        created_token: user_token_info.token_value,
    }))
}

async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let id = id.into_inner();
    let user_token = UserToken::find(id).await?;
    get_token_user_id_and_verify_permissions(&identity, Some(user_token.user_id)).await?;
    user_token.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(get_all));
    cfg.route("/{id}", web::get().to(get));
    cfg.route("", web::post().to(create));
    cfg.route("/{id}", web::delete().to(delete));
}
