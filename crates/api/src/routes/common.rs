//! Общи helpers на рутерите: активна фирма и потребител.

use axum::http::StatusCode;

use storage::{companies, models::User, users};

use crate::{errors::ApiError, state::AppState};

/// Активната фирма на потребителя; без членство — 403.
pub async fn active_company(
    st: &AppState,
    sub: &str,
) -> Result<(User, storage::models::Company), ApiError> {
    let mut conn = st
        .pool
        .get()
        .await
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "вътрешна грешка"))?;
    let user = users::by_login(&mut conn, sub)
        .await?
        .ok_or_else(|| ApiError::unauthorized("потребителят не съществува"))?;
    let row = companies::resolve_active(&mut conn, sub, user.id)
        .await?
        .ok_or_else(|| ApiError::forbidden("нямате достъп до фирма"))?;
    Ok((user, row.0))
}

/// Пагиниране в паметта (лимит 50 по подразбиране, макс 500).
#[derive(serde::Deserialize)]
pub struct PageQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

impl PageQuery {
    pub fn limit(&self) -> i64 {
        self.limit.clamp(1, 500)
    }
    pub fn offset(&self) -> i64 {
        self.offset.max(0)
    }
}

pub struct PageSlice<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl<T> PageSlice<T> {
    pub fn from_vec(mut all: Vec<T>, q: &PageQuery) -> Self {
        let total = all.len() as i64;
        let offset = q.offset().min(total);
        let limit = q.limit();
        let items: Vec<T> = all.drain(offset as usize..).take(limit as usize).collect();
        Self {
            items,
            total,
            limit,
            offset,
        }
    }
    pub fn count(&self) -> i64 {
        self.items.len() as i64
    }
    pub fn has_more(&self) -> bool {
        self.offset + self.count() < self.total
    }
}
