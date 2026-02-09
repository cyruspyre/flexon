//! Borrowed JSON value representation.

mod string;

use core::ops::{Index, IndexMut};

use crate::{pointer::JsonPointer, value::misc::define_value};

pub use string::String;

define_value! {
    name: Value,
    key_str: String,
    val_str: String,
}

impl Value {
    /// Returns `()` if it is a null, `None` otherwise.
    #[inline]
    pub fn as_null(&self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            _ => None,
        }
    }

    /// Returns `bool` if it is a boolean, `None` otherwise.
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns a reference to [`Array`] if it is an array, `None` otherwise.
    #[inline]
    pub fn as_array(&self) -> Option<&Array<Self>> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a mutable reference to [`Array`] if it is an array, `None` otherwise.
    #[inline]
    pub fn as_array_mut(&mut self) -> Option<&mut Array<Self>> {
        match self {
            Self::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a reference to [`Object`] if it is an object, `None` otherwise.
    #[inline]
    pub fn as_object(&self) -> Option<&Object<String, Value>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Returns a mutable reference to [`Object`] if it is an object, `None` otherwise.
    #[inline]
    pub fn as_object_mut(&mut self) -> Option<&mut Object<String, Value>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Returns [`Number`] if it is a number, `None` otherwise.
    #[inline]
    pub fn as_number(&self) -> Option<Number> {
        match *self {
            Self::Number(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `i64` if it is an integer and negative, `None` otherwise.
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number()?.as_i64()
    }

    /// Returns `i64` if it is an integer and positive, `None` otherwise.
    #[inline]
    pub fn as_u64(&self) -> Option<u64> {
        self.as_number()?.as_u64()
    }

    /// Returns `f64` if it is a floating point number or an integer that is too big, `None` otherwise.
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number()?.as_f64()
    }

    /// Returns string slice if it is a string, `None` otherwise.
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `true` if it is a string, `false` otherwise.
    #[inline]
    pub fn is_str(&self) -> bool {
        self.as_str().is_some()
    }

    /// Returns `true` if it is a null, `false` otherwise.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.as_null().is_some()
    }

    /// Returns `true` if it is a boolean, `false` otherwise.
    #[inline]
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// Returns `true` if it is an array, `false` otherwise.
    #[inline]
    pub fn is_array(&self) -> bool {
        self.as_array().is_some()
    }

    /// Returns `true` if it is an object, `false` otherwise.
    #[inline]
    pub fn is_object(&self) -> bool {
        self.as_object().is_some()
    }

    /// Returns `true` if it is a number, `false` otherwise.
    #[inline]
    pub fn is_number(&self) -> bool {
        self.as_number().is_some()
    }

    /// Returns `true` if it is an integer and negative, `false` otherwise.
    #[inline]
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Returns `true` if it is an integer and positive, `false` otherwise.
    #[inline]
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// Returns `true` if it is a floating point number or an integer that is too big, `false` otherwise.
    #[inline]
    pub fn is_f64(&self) -> bool {
        self.as_f64().is_some()
    }

    /// Looks up a value by the given path and returns a reference.
    ///
    /// If the path is empty, then the root value is returned.
    ///
    /// # Example
    /// ```
    /// use flexon::{Value, jsonp};
    ///
    /// let val: Value = flexon::parse(r#"{"foo": ["bar", 123]}"#)?;
    ///
    /// assert!(val.pointer(jsonp!["foo", 1]).unwrap().is_number());
    /// assert!(val.pointer([0]).is_none());
    ///
    /// # Ok::<_, flexon::Error>(())
    /// ```
    pub fn pointer<P>(&self, p: P) -> Option<&Value>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp {
                Value::Object(obj) => obj.get(pointer.as_key()?),
                Value::Array(arr) => arr.get(pointer.as_index()?),
                _ => None,
            }?
        }

        Some(tmp)
    }

    /// Looks up a value by the given path and returns a mutable reference.
    pub fn pointer_mut<P>(&mut self, p: P) -> Option<&mut Value>
    where
        P: IntoIterator,
        P::Item: JsonPointer,
    {
        let mut tmp = self;

        for pointer in p {
            tmp = match tmp {
                Value::Object(obj) => obj.get_mut(pointer.as_key()?),
                Value::Array(arr) => arr.get_mut(pointer.as_index()?),
                _ => None,
            }?
        }

        Some(tmp)
    }
}

impl Index<usize> for Value {
    type Output = Value;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        match self.as_array() {
            Some(v) => match v.get(idx) {
                Some(v) => v,
                _ => panic!("given index does not exist in the array"),
            },
            _ => panic!("value is not an array"),
        }
    }
}

impl IndexMut<usize> for Value {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match self.as_array_mut() {
            Some(v) => match v.get_mut(idx) {
                Some(v) => v,
                _ => panic!("given index does not exist in the array"),
            },
            _ => panic!("value is not an array"),
        }
    }
}

impl Index<&str> for Value {
    type Output = Value;

    #[inline]
    fn index(&self, key: &str) -> &Self::Output {
        match self.as_object() {
            Some(v) => match v.get(key) {
                Some(v) => v,
                _ => panic!("given key does not exist in the object"),
            },
            _ => panic!("value is not an object"),
        }
    }
}

impl IndexMut<&str> for Value {
    #[inline]
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        match self.as_object_mut() {
            Some(v) => match v.get_mut(key) {
                Some(v) => v,
                _ => panic!("given key does not exist in the object"),
            },
            _ => panic!("value is not an object"),
        }
    }
}
