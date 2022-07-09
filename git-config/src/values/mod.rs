mod string;
pub use string::String;

///
pub mod boolean;
///
pub mod color;
///
pub mod integer;
///
pub mod path;

/// Any value that may contain a foreground color, background color, a
/// collection of color (text) modifiers, or a combination of any of the
/// aforementioned values, like `red` or `brightgreen`.
///
/// Note that `git-config` allows color values to simply be a collection of
/// [`color::Attribute`]s, and does not require a [`color::Name`] for either the
/// foreground or background color.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Color {
    /// A provided foreground color
    pub foreground: Option<color::Name>,
    /// A provided background color
    pub background: Option<color::Name>,
    /// A potentially empty set of text attributes
    pub attributes: color::Attribute,
}

/// Any value that can be interpreted as an integer.
///
/// This supports any numeric value that can fit in a [`i64`], excluding the
/// suffix. The suffix is parsed separately from the value itself, so if you
/// wish to obtain the true value of the integer, you must account for the
/// suffix after fetching the value. [`integer::Suffix`] provides
/// [`bitwise_offset()`][integer::Suffix::bitwise_offset] to help with the
/// math, or [to_decimal()][Integer::to_decimal()] for obtaining a usable value in one step.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Integer {
    /// The value, without any suffix modification
    pub value: i64,
    /// A provided suffix, if any.
    pub suffix: Option<integer::Suffix>,
}

/// Any value that can be interpreted as a boolean.
///
/// Note that while values can effectively be any byte string, the `git-config`
/// documentation has a strict subset of values that may be interpreted as a
/// boolean value, all of which are ASCII and thus UTF-8 representable.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[allow(missing_docs)]
pub struct Boolean(pub bool);

/// Any value that can be interpreted as a file path.
///
/// Git represents file paths as byte arrays, modeled here as owned or borrowed byte sequences.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Path<'a> {
    /// The path string, un-interpolated
    pub value: std::borrow::Cow<'a, bstr::BStr>,
}
