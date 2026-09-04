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

    #[doc(hidden)]
    fn depth(&mut self) -> &mut impl Depth;
}

/// Represents the semantic depth of a JSON.
///
/// This is different from call stack depth as it solely reflects the
/// structural nesting of the document, i.e., how many arrays or objects are open.
///
/// # Example
/// ```
/// # use flexon::{Parser, Value, config::CTConfig};
/// let cfg = CTConfig::new().depth_limit(0);
///
/// assert!(Parser::new_with("123", cfg).parse::<Value>().is_ok());
/// assert!(Parser::new_with("[]", cfg).parse::<Value>().is_err());
/// ```
pub trait Depth {
    /// Returns a new instance in its initial state.
    fn new() -> Self;

    /// Called when the parser enters an array or object.
    fn increase(&mut self);

    /// Called when the parser finishes parsing an array or object.
    fn decrease(&mut self);

    /// Returns `true` when the depth limit has been reached.
    /// Additionally, the parser will not call [`Self::increase`].
    fn is_limit_reached(&self) -> bool;
}

/// Unbounded depth for JSON.
pub struct Unbounded;

impl Depth for Unbounded {
    #[inline]
    fn new() -> Self {
        Unbounded
    }

    #[inline]
    fn increase(&mut self) {}

    #[inline]
    fn decrease(&mut self) {}

    #[inline]
    fn is_limit_reached(&self) -> bool {
        false
    }
}

impl Depth for u8 {
    #[inline]
    fn new() -> Self {
        u8::MAX
    }

    #[inline]
    fn increase(&mut self) {
        *self -= 1
    }

    #[inline]
    fn decrease(&mut self) {
        *self += 1
    }

    #[inline]
    fn is_limit_reached(&self) -> bool {
        *self == 0
    }
}

/// Runtime configuration for JSON parsing behavior.
///
/// Useful when you don't care about performance or want to reduce build size/time.
#[derive(Clone, Copy)]
pub struct RTConfig<T = u8> {
    comma: bool,
    trailing_comma: bool,
    #[cfg(feature = "comment")]
    comments: bool,
    depth: T,
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
            depth: Depth::new(),
        }
    }
}

impl<T: Depth> RTConfig<T> {
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

    /// Sets depth limit for the input JSON.
    #[inline]
    pub fn depth_limit<U>(self, depth: U) -> RTConfig<U> {
        RTConfig {
            comma: self.comma,
            trailing_comma: self.trailing_comma,
            #[cfg(feature = "comment")]
            comments: self.comments,
            depth,
        }
    }
}

impl<T: Depth> Config for RTConfig<T> {
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

    #[inline]
    fn depth(&mut self) -> &mut impl Depth {
        &mut self.depth
    }
}

impl<T> Sealed for RTConfig<T> {}

/// Compile-time configuration for JSON parsing behavior.
#[derive(Clone, Copy)]
pub struct CTConfig<
    T = u8,
    const REQUIRE_COMMA: bool = true,
    const TRAILING_COMMA: bool = false,
    const PRE_VALIDATE_UTF8: bool = true,
    #[cfg(feature = "comment")] const COMMENTS: bool = false,
> {
    depth: T,
}

impl CTConfig {
    /// Creates a compile-time configuration with default settings.
    ///
    /// By default, commas are required and both trailing commas and comments are not allowed.
    pub fn new() -> Self {
        Self {
            depth: Depth::new(),
        }
    }
}

impl<T, const A: bool, const B: bool, const C: bool> CTConfig<T, A, B, C> {
    /// Allows comments when parsing.
    #[cfg(feature = "comment")]
    pub fn allow_comments(self) -> CTConfig<T, A, B, C, true> {
        CTConfig { depth: self.depth }
    }
}

#[cfg(feature = "comment")]
#[cfg_attr(docsrs, doc(cfg(all())))]
mod __ {
    use crate::config::Depth;

    use super::{CTConfig, Config, Sealed};

    impl<T, const A: bool, const B: bool, const C: bool> CTConfig<T, true, A, B, C> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        pub fn optional_comma(self) -> CTConfig<T, false, true, B, C> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool> CTConfig<T, A, false, B, C> {
        /// Allows trailing commas when parsing.
        pub fn allow_trailing_comma(self) -> CTConfig<T, A, true, B, C> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool> CTConfig<T, A, B, true, C> {
        /// Disables UTF-8 pre-validation, which would otherwise be performed eagerly when possible.
        ///
        /// Performing a single UTF-8 validation over the whole source is usually faster than doing it
        /// for each JSON string that is parsed. However, it can be considered *overhead* when the JSON
        /// has little to no strings.
        ///
        /// Without this, the default behavior is to pre-validate the entire source for UTF-8 when
        /// the source is non-volatile and not guaranteed to be valid UTF-8 already.
        pub fn disable_utf8_pre_validation(self) -> CTConfig<T, A, B, false, C> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool, const D: bool> CTConfig<T, A, B, C, D> {
        /// Sets depth limit for the input JSON.
        #[inline]
        pub fn depth_limit<U>(self, v: U) -> CTConfig<U, A, B, C, D> {
            CTConfig { depth: v }
        }
    }

    impl<
        T: Depth,
        const REQUIRE_COMMA: bool,
        const TRAILING_COMMA: bool,
        const PRE_VALIDATE_UTF8: bool,
        const COMMENTS: bool,
    > Config for CTConfig<T, REQUIRE_COMMA, TRAILING_COMMA, PRE_VALIDATE_UTF8, COMMENTS>
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

        #[inline]
        fn depth(&mut self) -> &mut impl Depth {
            &mut self.depth
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool, const D: bool> Sealed
        for CTConfig<T, A, B, C, D>
    {
    }
}

#[cfg(not(feature = "comment"))]
mod __ {
    use super::{CTConfig, Config, Depth, Sealed};

    impl<T, const A: bool, const B: bool> CTConfig<T, true, A, B> {
        /// Makes commas optional. As a side effect trailing commas are allowed automatically.
        pub fn optional_comma(self) -> CTConfig<T, false, true, B> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool> CTConfig<T, true, false, A> {
        /// Allows trailing commas when parsing.
        pub fn allow_trailing_comma(self) -> CTConfig<T, true, true, A> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool, const B: bool> CTConfig<T, A, B, true> {
        /// Disables UTF-8 pre-validation, which would otherwise be performed eagerly when possible.
        ///
        /// Performing a single UTF-8 validation over the whole source is usually faster than doing it
        /// for each JSON string that is parsed. However, it can be considered *overhead* when the JSON
        /// has little to no strings.
        ///
        /// Without this, the default behavior is to pre-validate the entire source for UTF-8 when
        /// the source is non-volatile and not guaranteed to be valid UTF-8 already.
        pub fn disable_utf8_pre_validation(self) -> CTConfig<T, A, B, false> {
            CTConfig { depth: self.depth }
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool> CTConfig<T, A, B, C> {
        /// Sets depth limit for the input JSON.
        #[inline]
        pub fn depth_limit<U>(self, v: U) -> CTConfig<U, A, B, C> {
            CTConfig { depth: v }
        }
    }

    impl<
        T: Depth,
        const REQUIRE_COMMA: bool,
        const TRAILING_COMMA: bool,
        const PRE_VALIDATE_UTF8: bool,
    > Config for CTConfig<T, REQUIRE_COMMA, TRAILING_COMMA, PRE_VALIDATE_UTF8>
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

        #[inline]
        fn depth(&mut self) -> &mut impl super::Depth {
            &mut self.depth
        }
    }

    impl<T, const A: bool, const B: bool, const C: bool> Sealed for CTConfig<T, A, B, C> {}
}
