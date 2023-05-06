use serde::Serialize;
use utoipa::ToSchema;

pub const DEFAULT_PAGE_SIZE: i64 = 50;

#[derive(Serialize, ToSchema)]
pub struct Paginated<T>
where
    T: Serialize + ToSchema<'static>,
{
    #[schema(example = "1")]
    pub page: i64,
    #[schema(example = "50")]
    pub per_page: i64,
    #[schema(inline, value_type=[T])]
    pub items: Vec<T>,
    #[schema(example = "1")]
    pub num_total: i64,
}
