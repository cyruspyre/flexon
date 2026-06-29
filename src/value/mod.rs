//! JSON value types and utilities.

pub mod builder;
pub(crate) mod number;

#[cfg(feature = "alloc")]
mod array;
#[cfg(feature = "alloc")]
pub mod borrowed;
#[cfg(feature = "alloc")]
pub mod lazy;
#[cfg(feature = "alloc")]
mod misc;
#[cfg(feature = "alloc")]
mod object;
#[cfg(feature = "alloc")]
pub mod owned;

pub use number::Number;

#[cfg(feature = "alloc")]
pub use array::Array;
#[cfg(feature = "alloc")]
pub use object::Object;

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use borrowed::Value;
#[cfg(feature = "alloc")]
#[doc(inline)]
pub use lazy::Value as LazyValue;
#[doc(inline)]
#[cfg(feature = "alloc")]
pub use owned::Value as OwnedValue;
