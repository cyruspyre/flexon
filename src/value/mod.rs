//! JSON value types and utilities.

pub mod builder;
pub(crate) mod number;

pub use number::Number;

cfg_select! {
    feature = "alloc" => {
        mod array;
        pub mod borrowed;
        pub mod lazy;
        mod misc;
        mod object;
        pub mod owned;

        pub use array::Array;
        pub use object::Object;

        #[doc(inline)]
        pub use borrowed::Value;
        #[doc(inline)]
        pub use lazy::Value as LazyValue;
        #[doc(inline)]
        pub use owned::Value as OwnedValue;
    }
    _ => {}
}
