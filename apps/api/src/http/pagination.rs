use serde::{Deserialize, Serialize};

const DEFAULT_PER_PAGE: u32 = 20;
const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct PageParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl PageParams {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE)
    }

    pub fn offset(&self) -> i64 {
        i64::from(self.page() - 1) * i64::from(self.per_page())
    }

    pub fn limit(&self) -> i64 {
        i64::from(self.per_page())
    }
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, params: PageParams, total: u64) -> Self {
        Self {
            items,
            page: params.page(),
            per_page: params.per_page(),
            total,
        }
    }

    /// Paginates an already filtered in-memory collection.
    pub fn slice(all: Vec<T>, params: PageParams) -> Self {
        let total = all.len() as u64;
        let items = all
            .into_iter()
            .skip(params.offset() as usize)
            .take(params.per_page() as usize)
            .collect();
        Self::new(items, params, total)
    }
}
