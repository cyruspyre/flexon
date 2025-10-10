use std::{fmt::Debug, hash::Hash, ops::Deref};

pub struct Span<T> {
    pub(crate) data: T,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl<T> Span<T> {
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
}

impl<T> Deref for Span<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: PartialOrd> PartialOrd for Span<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T: Ord> Ord for Span<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl<T: Hash> Hash for Span<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<T: PartialEq> PartialEq for Span<T> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: Eq> Eq for Span<T> {}

impl<T: Clone> Clone for Span<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
        }
    }
}

impl<T: Copy> Copy for Span<T> {}

impl<T: Debug> Debug for Span<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            return self.data.fmt(f);
        }

        f.debug_struct("Span")
            .field("data", &self.data)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}
