use super::utils::ensure_user;
use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, web, HttpResponse};
use platz_auth::{generate_api_token, ApiIdentity};
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::bot_token::{BotToken, BotTokenFilters, NewBotToken},
    DbError,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bot Tokens",
    operation_id = "allBotTokens",
    security(
        ("access_token" = []),
    ),
    params(BotTokenFilters),
    responses(
        (
            status = OK,
            body = Paginated<BotToken>,
        ),
    ),
)]
#[get("/bot-tokens")]
async fn get_all(
    identity: ApiIdentity,
    filters: web::Query<BotTokenFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    Ok(HttpResponse::Ok()
        .json(BotToken::all_filtered(filters.into_inner(), pagination.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bot Tokens",
    operation_id = "getBotToken",
    security(
        ("access_token" = []),
    ),
    responses(
        (
            status = OK,
            body = BotToken,
        ),
    ),
)]
#[get("/bot-tokens/{id}")]
async fn get_one(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let bot_token = BotToken::find(id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(bot_token))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBotToken {
    pub bot_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedBotToken {
    created_token: String,
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bot Tokens",
    operation_id = "createBotToken",
    security(
        ("access_token" = []),
    ),
    request_body = CreateBotToken,
    responses(
        (
            status = CREATED,
            body = CreatedBotToken,
        ),
    ),
)]
#[post("/bot-tokens")]
async fn create(identity: ApiIdentity, body: web::Json<CreateBotToken>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let user = ensure_user(&identity).await?;
    let body = body.into_inner();
    let api_token_info = generate_api_token().await?;
    NewBotToken {
        id: api_token_info.token_id,
        bot_id: body.bot_id,
        created_by_user_id: user.id,
        secret_hash: api_token_info.secret_hash,
    }
    .save()
    .await?;

    Ok(HttpResponse::Created().json(CreatedBotToken {
        created_token: api_token_info.token_value,
    }))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bot Tokens",
    operation_id = "deleteBotToken",
    security(
        ("access_token" = []),
    ),
    responses(
        (
            status = NO_CONTENT,
        ),
    ),
)]
#[delete("/bot-tokens/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    let Some(bot_token) = BotToken::find(id).await? else {
        return Err(DbError::NotFound)?;
    };
    bot_token.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Bot Tokens",
        description = "\
Bot tokens allow bots to authenticate using a long-lived token that can be
used in direct API calls, CLI, etc.

Bot tokens are passed in the `x-platz-token` header.
",
    )),
    paths(get_all, get_one, create, delete),
)]
pub(super) struct OpenApi;
