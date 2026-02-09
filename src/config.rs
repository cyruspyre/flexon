//! Configuration types for customizing JSON parsing behavior.

use crate::misc::Sealed;

/// Configuration trait for JSON parsing behavior.
pub trait Config: Sealed {
    #[doc(hidden)]
    fn comma(&self) -> bool;

    #[doc(hidden)]
    fn trailing_comma(&self) -> bool;

    #[doc(hidden)]
    #[cfg(feature = "comment")]
    fn comments(&self) -> bool;
}

/// Runtime configuration for JSON parsing behavior.
///
/// Useful when you don't care about performance or want to reduce build size/time.
pub struct RTConfig {
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "comment")]
    comments: bool,
}

impl RTConfig {
    /// Creates a runtime configuration with default settings.
    ///
    /// By default, commas are required and trailing commas are not allowed.
    pub fn new() -> Self {
        Self {
            comma: false,
            trailing_comma: false,
            #[cfg(feature = "comment")]
            comments: false,
        }
    }

    /// Sets whether commas are required or not.
    ///
    /// When set to `true`, commas are mandatory. When `false`, commas
    /// are optional and trailing commas are allowed.
    pub fn require_comma(mut self, v: bool) -> Self {
        self.trailing_comma = !v;
        self.comma = !v;
        self
    }

    /// Sets whether trailing commas are allowed or not.
    ///
    /// Has no effect when commas are optional.
    pub fn allow_trailing_comma(mut self, v: bool) -> Self {
        self.trailing_comma = v | self.comma;
        self
    }

    /// Sets whether comments are allowed or not.
    #[cfg(feature = "comment")]
    pub fn allow_comments(mut self, v: bool) -> Self {
        self.comments = v;
        self
    }
}

impl Config for RTConfig {
    #[inline(always)]
    fn comma(&self) -> bool {
        self.comma
    }

    #[inline(always)]
    fn trailing_comma(&self) -> bool {
        self.trailing_comma
    }

    #[inline(always)]
    #[cfg(feature = "comment")]
    fn comments(&self) -> bool {
        self.comments
    }
}

impl Sealed for RTConfig {}

/// Compile-time configuration for JSON parsing behavior.
pub struct CTConfig<
    const COMMA: bool = true,
    const TRAILING_COMMA: bool = false,
    #[cfg(feature = "comment")] const COMMENTS: bool = false,
>;

impl CTConfig {
    /// Creates a compile-time configuration with default settings.
    ///
    /// By default, commas are required and both trailing commas and comments are not allowed.
    pub fn new() -> Self {
        Self
    }
}

impl<const A: bool, const B: bool> CTConfig<A, B> {
    /// Allows comments when parsing.
    #[inline]
    #[cfg(feature = "comment")]
    pub fn allow_comments(self) -> CTConfig<A, B, true> {
        CTConfig
    }
}

#[cfg(feature = "comment")]
// #[doc(cfg(all(not(feature = "comment"), feature = "comment")))]
mod __ {
    use super::{CTConfig, Config, Sealed};

    impl<const A: bool, const B: bool> CTConfig<true, A, B> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        #[inline]
        pub fn optional_comma(self) -> CTConfig<false, true, B> {
            CTConfig
        }
    }

    impl<const A: bool, const B: bool> CTConfig<A, false, B> {
        /// Allows trailing commas when parsing.
        #[inline]
        pub fn allow_trailing_comma(self) -> CTConfig<A, true, B> {
            CTConfig
        }
    }

    impl<const COMMA: bool, const TRAILING_COMMA: bool, const COMMENTS: bool> Config
        for CTConfig<COMMA, TRAILING_COMMA, COMMENTS>
    {
        #[inline(always)]
        fn comma(&self) -> bool {
            !COMMA
        }

        #[inline(always)]
        fn trailing_comma(&self) -> bool {
            TRAILING_COMMA | !COMMA
        }

        #[inline(always)]
        fn comments(&self) -> bool {
            COMMENTS
        }
    }

    impl<const A: bool, const B: bool, const C: bool> Sealed for CTConfig<A, B, C> {}
}

#[cfg(not(feature = "comment"))]
mod __ {
    use super::{CTConfig, Config, Sealed};

    impl<const A: bool> CTConfig<true, A> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        #[inline]
        pub fn optional_comma(self) -> CTConfig<false, true> {
            CTConfig
        }
    }

    impl<const V: bool> CTConfig<V> {
        /// Allows trailing commas when parsing.
        #[inline]
        pub fn allow_trailing_comma(self) -> CTConfig<V, true> {
            CTConfig
        }
    }

    impl<const COMMA: bool, const TRAILING_COMMA: bool> Config for CTConfig<COMMA, TRAILING_COMMA> {
        #[inline(always)]
        fn comma(&self) -> bool {
            !COMMA
        }

        #[inline(always)]
        fn trailing_comma(&self) -> bool {
            TRAILING_COMMA | !COMMA
        }
    }

    impl<const A: bool, const B: bool> Sealed for CTConfig<A, B> {}
}
