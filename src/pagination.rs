use crate::error::AppError;
use serde::Deserialize;
#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "one")]
    pub page: i64,
    #[serde(default = "twenty")]
    pub limit: i64,
}
fn one() -> i64 {
    1
}
fn twenty() -> i64 {
    20
}
impl Pagination {
    pub fn offset(&self) -> Result<i64, AppError> {
        if !(1..=100).contains(&self.limit) || !(1..=1_000_000).contains(&self.page) {
            return Err(AppError::bad_request(
                "page must be 1–1000000; limit must be 1–100",
            ));
        }
        Ok((self.page - 1) * self.limit)
    }
}
