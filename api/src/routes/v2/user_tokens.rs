use crate::permissions::verify_site_admin;
use crate::result::{ApiError, ApiResult};
use actix_web::{web, HttpResponse};
use platz_auth::{generate_user_token, ApiIdentity};
use platz_db::{Identity, NewUserToken, UserToken, UserTokenFilters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

async fn get_all(_identity: ApiIdentity, filters: web::Query<UserTokenFilters>) -> ApiResult {
    Ok(HttpResponse::Ok().json(UserToken::all_filtered(filters.into_inner()).await?))
}

async fn get(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(UserToken::find(id.into_inner()).await?))
}

async fn get_token_user_id_and_verify_permissions(
    identity: &Identity,
    token_for_user_id: Option<Uuid>,
) -> Result<Uuid, ApiError> {
    match identity.user_id() {
        None => Err(ApiError::NoPermission),
        Some(identity_user_id) => {
            let token_user_id = token_for_user_id.unwrap_or(identity_user_id);
            if token_user_id != identity_user_id {
                // Only site admins can handle tokens for other users
                verify_site_admin(identity).await?;
            }
            Ok(token_user_id)
        }
    }
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
    let identity = identity.inner();
    let token_user_id =
        get_token_user_id_and_verify_permissions(identity, new_user_token.user_id).await?;

    let user_token_info = generate_user_token();
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
    let identity = identity.into_inner();
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
