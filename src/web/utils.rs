/// Helper to convert empty strings to `None` values
pub trait EmptyToNone<T> {
    fn empty_to_none(&self) -> Option<T>;
}

impl EmptyToNone<String> for Option<String> {
    fn empty_to_none(&self) -> Option<String> {
        match self {
            None => None,
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s.clone())
                }
            }
        }
    }
}

/// Helper to deserialize search queries
#[derive(Deserialize)]
pub struct Search {
    pub search: Option<String>,
}
