//! Configuration types for customizing JSON parsing behavior.

use crate::misc::Sealed;

/// Configuration trait for JSON parsing behavior.
pub trait Config: Sealed {
    #[doc(hidden)]
    const PRE_VALIDATE_UTF8: bool;

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
    const PRE_VALIDATE_UTF8: bool = true;

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
    const REQUIRE_COMMA: bool = true,
    const TRAILING_COMMA: bool = false,
    const PRE_VALIDATE_UTF8: bool = true,
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

impl<const A: bool, const B: bool, const C: bool> CTConfig<A, B, C> {
    /// Allows comments when parsing.
    #[cfg(feature = "comment")]
    pub fn allow_comments(self) -> CTConfig<A, B, C, true> {
        CTConfig
    }
}

#[cfg(feature = "comment")]
#[cfg_attr(docsrs, doc(cfg(all())))]
mod __ {
    use super::{CTConfig, Config, Sealed};

    impl<const A: bool, const B: bool, const C: bool> CTConfig<true, A, B, C> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        pub fn optional_comma(self) -> CTConfig<false, true, B, C> {
            CTConfig
        }
    }

    impl<const A: bool, const B: bool, const C: bool> CTConfig<A, false, B, C> {
        /// Allows trailing commas when parsing.
        pub fn allow_trailing_comma(self) -> CTConfig<A, true, B, C> {
            CTConfig
        }
    }

    impl<const A: bool, const B: bool, const C: bool> CTConfig<A, B, true, C> {
        /// Disables UTF-8 pre-validation, which would otherwise be performed eagerly when possible.
        ///
        /// Performing a single UTF-8 validation over the whole source is usually faster than doing it
        /// for each JSON string that is parsed. However, it can be considered *overhead* when the JSON
        /// has little to no strings.
        ///
        /// Without this, the default behavior is to pre-validate the entire source for UTF-8 when
        /// the source is non-volatile and not guaranteed to be valid UTF-8 already.
        pub fn disable_utf8_pre_validation(self) -> CTConfig<A, B, false, C> {
            CTConfig
        }
    }

    impl<
        const REQUIRE_COMMA: bool,
        const TRAILING_COMMA: bool,
        const PRE_VALIDATE_UTF8: bool,
        const COMMENTS: bool,
    > Config for CTConfig<REQUIRE_COMMA, TRAILING_COMMA, PRE_VALIDATE_UTF8, COMMENTS>
    {
        const PRE_VALIDATE_UTF8: bool = PRE_VALIDATE_UTF8;

        #[inline(always)]
        fn comma(&self) -> bool {
            !REQUIRE_COMMA
        }

        #[inline(always)]
        fn trailing_comma(&self) -> bool {
            TRAILING_COMMA | !REQUIRE_COMMA
        }

        #[inline(always)]
        fn comments(&self) -> bool {
            COMMENTS
        }
    }

    impl<const A: bool, const B: bool, const C: bool, const D: bool> Sealed for CTConfig<A, B, C, D> {}
}

#[cfg(not(feature = "comment"))]
mod __ {
    use super::{CTConfig, Config, Sealed};

    impl<const A: bool, const B: bool> CTConfig<true, A, B> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        pub fn optional_comma(self) -> CTConfig<false, true, B> {
            CTConfig
        }
    }

    impl<const A: bool> CTConfig<true, false, A> {
        /// Allows trailing commas when parsing.
        pub fn allow_trailing_comma(self) -> CTConfig<true, true, A> {
            CTConfig
        }
    }

    impl<const A: bool, const B: bool> CTConfig<A, B, true> {
        /// Disables UTF-8 pre-validation, which would otherwise be performed eagerly when possible.
        ///
        /// Performing a single UTF-8 validation over the whole source is usually faster than doing it
        /// for each JSON string that is parsed. However, it can be considered *overhead* when the JSON
        /// has little to no strings.
        ///
        /// Without this, the default behavior is to pre-validate the entire source for UTF-8 when
        /// the source is non-volatile and not guaranteed to be valid UTF-8 already.
        pub fn disable_utf8_pre_validation(self) -> CTConfig<A, B, false> {
            CTConfig
        }
    }

    impl<const REQUIRE_COMMA: bool, const TRAILING_COMMA: bool, const PRE_VALIDATE_UTF8: bool>
        Config for CTConfig<REQUIRE_COMMA, TRAILING_COMMA, PRE_VALIDATE_UTF8>
    {
        const PRE_VALIDATE_UTF8: bool = PRE_VALIDATE_UTF8;

        #[inline(always)]
        fn comma(&self) -> bool {
            !REQUIRE_COMMA
        }

        #[inline(always)]
        fn trailing_comma(&self) -> bool {
            TRAILING_COMMA | !REQUIRE_COMMA
        }
    }

    impl<const A: bool, const B: bool, const C: bool> Sealed for CTConfig<A, B, C> {}
}
