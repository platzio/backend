use crate::permissions::verify_site_admin;
use crate::result::ApiResult;
use actix_web::{get, put, web, HttpResponse};
use platz_auth::ApiIdentity;
use platz_db::{Paginated, UpdateUser, User, UserExtraFilters, UserFilters};
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Users",
    operation_id = "allUsers",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    params(UserFilters),
    responses(
        (
            status = OK,
            body = inline(Paginated<User>),
        ),
    ),
)]
#[get("/users")]
async fn get_all(
    _identity: ApiIdentity,
    filters: web::Query<UserFilters>,
    extra_filters: web::Query<UserExtraFilters>,
) -> ApiResult {
    Ok(HttpResponse::Ok()
        .json(User::all_filtered(filters.into_inner(), extra_filters.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Users",
    operation_id = "getUser",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    responses(
        (
            status = OK,
            body = User,
        ),
    ),
)]
#[get("/users/{id}")]
async fn get_one(_identity: ApiIdentity, id: web::Path<Uuid>) -> ApiResult {
    Ok(HttpResponse::Ok().json(User::find(id.into_inner()).await?))
}

#[utoipa::path(
    context_path = "/api/v2",
    tag = "Users",
    operation_id = "updateUser",
    security(
        ("access_token" = []),
        ("user_token" = []),
    ),
    request_body = UpdateUser,
    responses(
        (
            status = OK,
            body = User,
        ),
    ),
)]
#[put("/users/{id}")]
async fn update(
    identity: ApiIdentity,
    id: web::Path<Uuid>,
    update: web::Json<UpdateUser>,
) -> ApiResult {
    verify_site_admin(&identity).await?;
    let id = id.into_inner();
    if identity.inner().user_id() == Some(id) {
        Ok(HttpResponse::Forbidden().json(json!({
            "message": "You can't update your own user"
        })))
    } else {
        Ok(HttpResponse::Ok().json(update.into_inner().save(id).await?))
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((
        name = "Users",
        description = "This collection contains all users in Platz.",
    )),
    paths(get_all, get_one, update),
    components(schemas(
        User,
        UpdateUser,
    )),
)]
pub(super) struct OpenApi;
