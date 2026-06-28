use std::iter::{Extend, IntoIterator};

#[derive(Debug, Default)]
pub struct QueryResult {
    pub(super) rows_affected: u64,
}

impl QueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }
}

impl Extend<QueryResult> for QueryResult {
    fn extend<T: IntoIterator<Item = QueryResult>>(&mut self, iter: T) {
        for elem in iter {
            self.rows_affected += elem.rows_affected;
        }
    }
}
