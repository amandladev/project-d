use serde::{Deserialize, Serialize};

/// Pagination request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    /// Maximum number of results to return.
    pub limit: usize,
    /// Number of results to skip.
    pub offset: usize,
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    /// The items for this page.
    pub items: Vec<T>,
    /// Total number of matching items (across all pages).
    pub total_count: usize,
    /// Whether there are more items after this page.
    pub has_more: bool,
}

impl<T> PaginatedResult<T> {
    /// Create a paginated result from a full list (in-memory pagination).
    pub fn from_vec(items: Vec<T>, total_count: usize, page: &PageRequest) -> Self {
        let has_more = page.offset + items.len() < total_count;
        Self {
            items,
            total_count,
            has_more,
        }
    }
}
