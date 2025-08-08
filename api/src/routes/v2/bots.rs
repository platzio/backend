use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{delete, get, post, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{
    diesel_pagination::{Paginated, PaginationParams},
    schema::bot::{Bot, BotFilters, NewBot, UpdateBot},
    DbError,
};
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bots",
    operation_id = "allBots",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(BotFilters),
    responses(
        (
            status = OK,
            body = Paginated<Bot>,
        ),
    ),
)]
#[get("/bots")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<BotFilters>,
    pagination: web::Query<PaginationParams>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(Bot::all_filtered(filters.into_inner(), pagination.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bots",
    operation_id = "getBot",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = Bot,
        ),
    ),
)]
#[get("/bots/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(Bot::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bots",
    operation_id = "createBot",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = NewBot,
    responses(
        (
            status = CREATED,
            body = Bot,
        ),
    ),
)]
#[post("/bots")]
async fn create(identity: ApiIdentity, new_bot: web::Json<NewBot>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let bot = new_bot.into_inner().insert().await?;
    Ok(HttpResponse::Created().json(bot))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bots",
    operation_id = "updateBot",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateBot,
    responses(
        (
            status = OK,
            body = Bot,
        ),
    ),
)]
#[put("/bots/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateBot>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    let bot = update.into_inner().save(id).await?;
    Ok(HttpResponse::Ok().json(bot))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Bots",
    operation_id = "deleteBot",
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
#[delete("/bots/{id}")]
async fn delete(identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    verify_site_admin(&identity).await?;
    let Some(bot) = Bot::find(id.into_inner()).await? else {
        return Err(DbError::NotFound)?;
    };
    bot.delete().await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Bots",
        description = "This collection contains all bots in Platz.",
    )),
    paths(get_all, get_one, update),
)]
pub(super) struct OpenApi;
