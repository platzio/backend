use super::utils::{ensure_user, ensure_user_id};
use crate::permissions::verify_site_admin;
use crate::result::{ApiError, ApiResult};
use actix_web::{delete, get, post, web, HttpResponse};
use platz_auth::{generate_user_token, ApiIdentity};
use platz_db::{DbError, NewUserToken, Paginated, UserToken, UserTokenFilters};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "User Tokens",
    operation_id = "allUserTokens",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(UserTokenFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<UserToken>),
        ),
    ),
)]
#[get("/user-tokens")]
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

#[utoipa::path(
    context_path = "/api/v2",
    tag = "User Tokens",
    operation_id = "getUserToken",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = UserToken,
        ),
    ),
)]
#[get("/user-tokens/{id}")]
async fn get_one(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserToken {
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedUserToken {
    created_token: String,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "User Tokens",
    operation_id = "createUserToken",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = CreateUserToken,
    responses(
        (
            status = CREATED,
            body = CreatedUserToken,
        ),
    ),
)]
#[post("/user-tokens")]
async fn create(identity: ApiIdentity, new_user_token: web::Json<CreateUserToken>) -> ApiResult {
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

    Ok(HttpResponse::Created().json(CreatedUserToken {
        created_token: user_token_info.token_value,
    }))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "User Tokens",
    operation_id = "deleteUserToken",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = NO_CONTENT,
        ),
    ),
)]
#[delete("/user-tokens/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    let id = id.into_inner();
    let user_token = UserToken::find(id).await?;
    get_token_user_id_and_verify_permissions(&identity, Some(user_token.user_id)).await?;
    user_token.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "User Tokens",
        description = "\
User tokens allow users to authenticate using a long-lived token that can be
used in direct API calls, CLI, etc.

User tokens are passed in the `x-platz-token` header.
",
    )),
    paths(get_all, get_one, create, delete),
    components(schemas(
        UserToken,
        CreateUserToken,
        CreatedUserToken,
    )),
)]
pub(super) struct OpenApi;
