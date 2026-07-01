//! JSON pointer representations.

pub enum Pointer<'a> {
    Key(&'a str),
    Index(usize),
}

impl JsonPointer for Pointer<'_> {
    #[inline]
    fn as_key(&self) -> Option<&str> {
        match self {
            Pointer::Key(v) => Some(v),
            _ => None,
        }
    }

    #[inline]
    fn as_index(&self) -> Option<usize> {
        match self {
            Pointer::Index(v) => Some(*v),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for Pointer<'a> {
    #[inline]
    fn from(value: &'a str) -> Self {
        Self::Key(value)
    }
}

impl From<usize> for Pointer<'_> {
    #[inline]
    fn from(value: usize) -> Self {
        Self::Index(value)
    }
}

/// A trait for types that can act as JSON pointers.
pub trait JsonPointer {
    /// Returns the pointer as a key if it represents an object property.
    fn as_key(&self) -> Option<&str>;

    /// Returns the pointer as an index if it represents an array index.
    fn as_index(&self) -> Option<usize>;
}

impl JsonPointer for &str {
    #[inline(always)]
    fn as_key(&self) -> Option<&str> {
        Some(self)
    }

    #[inline(always)]
    fn as_index(&self) -> Option<usize> {
        None
    }
}

impl JsonPointer for usize {
    #[inline(always)]
    fn as_key(&self) -> Option<&str> {
        None
    }

    #[inline(always)]
    fn as_index(&self) -> Option<usize> {
        Some(*self)
    }
}

/// Creates a representation of JSON pointers.
///
/// The path can either be a string for object keys or a number for array indices.
///
/// # Examples
/// ```
/// # use flexon::{jsonp, Value};
/// #
/// let src = r#"{"users": [{"name": "Walter"}]}"#;
/// let val: Value = flexon::parse_at(src, jsonp!["users", 0, "name"]).unwrap();
///
/// assert_eq!(val.as_str(), Some("Walter"));
/// ```
#[macro_export]
macro_rules! jsonp {
    () => (
        [] as [$crate::pointer::Pointer; 0]
    );

    ($($x:expr),+ $(,)?) => (
        [$($crate::pointer::Pointer::from($x)),+]
    );
}
