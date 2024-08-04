#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StorePath(Vec<StorePathSegment>);

impl IntoIterator for StorePath {
    type Item = StorePathSegment;
    type IntoIter = std::vec::IntoIter<StorePathSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<StorePathSegment>> for StorePath {
    fn from(value: Vec<StorePathSegment>) -> Self {
        Self(value)
    }
}

impl StorePath {
    pub fn push(&mut self, segment: impl Into<StorePathSegment>) {
        self.0.push(segment.into());
    }

    pub fn pop(&mut self) -> Option<StorePathSegment> {
        self.0.pop()
    }

    pub fn replace_last(&mut self, segment: impl Into<StorePathSegment>) {
        if let Some(last) = self.0.last_mut() {
            *last = segment.into();
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StorePathSegment(pub(crate) usize);

impl From<usize> for StorePathSegment {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<&usize> for StorePathSegment {
    fn from(value: &usize) -> Self {
        Self(*value)
    }
}

impl FromIterator<StorePathSegment> for StorePath {
    fn from_iter<T: IntoIterator<Item = StorePathSegment>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}
