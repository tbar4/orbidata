use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl PaginationParams {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page.unwrap_or(50).clamp(1, 500)
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(all_items: Vec<T>, page: u32, per_page: u32) -> Self {
        let total = all_items.len();
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        let start = ((page - 1) * per_page) as usize;
        let data = if start < total {
            all_items
                .into_iter()
                .skip(start)
                .take(per_page as usize)
                .collect()
        } else {
            vec![]
        };

        Self {
            data,
            meta: PaginationMeta {
                total,
                page,
                per_page,
                total_pages: total_pages.max(1),
            },
        }
    }
}
