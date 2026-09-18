
/// Paginated result for large archives.
#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: usize,
    pub total: Option<usize>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, offset: usize, total: Option<usize>) -> Self {
        Self {
            items,
            offset,
            total,
        }
    }
}
