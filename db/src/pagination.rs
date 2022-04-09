use serde::Serialize;

pub const DEFAULT_PAGE_SIZE: i64 = 50;

#[derive(Serialize)]
pub struct Paginated<T>
where
    T: Serialize,
{
    pub page: i64,
    pub per_page: i64,
    pub items: Vec<T>,
    pub num_total: i64,
}
