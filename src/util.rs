//! Utilities
use alloc::borrow::{Cow, ToOwned};
use alloc::string::String;
use core::fmt::{Display, Formatter, Result};

/// Helper type to write indented things
pub struct Indented<TDisplay>(pub TDisplay)
where
    TDisplay: Display;
impl<TDisplay> Display for Indented<TDisplay>
where
    TDisplay: Display,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "  ")?;
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod test_indented {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_write_indented() {
        assert_eq!("  foo", Indented("foo").to_string());
        assert_eq!("  ", Indented("".to_string()).to_string());
    }
}

/// Escape a string for ninja, without escaping spaces or colons
///
/// See <https://ninja-build.org/manual.html#ref_lexer>
///
/// # Examples
/// ```rust
/// use ninja_writer::escape;
///
/// assert_eq!(escape("$foo"), "$$foo");
/// assert_eq!(escape("foo bar"), "foo bar");
/// assert_eq!(escape("foo: bar"), "foo: bar");
/// ```
#[inline]
pub fn escape(s: &str) -> Cow<'_, str> {
    escape_impl(s, false, false)
}

/// Escape a string for ninja, including spaces in the string, but not colons
///
/// This is necessary when writing a list of paths, since spaces are used as separators.
/// See <https://ninja-build.org/manual.html#ref_lexer>
///
/// # Examples
/// ```rust
/// use ninja_writer::escape_path;
///
/// assert_eq!(escape_path("$foo"), "$$foo");
/// assert_eq!(escape_path("foo bar"), "foo$ bar");
/// assert_eq!(escape_path("foo: bar"), "foo:$ bar");
/// ```
#[inline]
pub fn escape_path(s: &str) -> Cow<'_, str> {
    escape_impl(s, true, false)
}

/// Escape a string for ninja. Spaces and colons are escaped as well
///
/// This is necessary when writing build outputs.
/// See <https://ninja-build.org/manual.html#ref_lexer>
///
/// # Examples
/// ```rust
/// use ninja_writer::escape_build;
///
/// assert_eq!(escape_build("$foo"), "$$foo");
/// assert_eq!(escape_build("foo bar"), "foo$ bar");
/// assert_eq!(escape_build("foo: bar"), "foo$:$ bar");
/// ```
#[inline]
pub fn escape_build(s: &str) -> Cow<'_, str> {
    escape_impl(s, true, true)
}

/// Implementation of escape
pub fn escape_impl(s: &str, escape_space: bool, escape_colon: bool) -> Cow<'_, str> {
    let mut output: Option<String> = None;
    for (i, c) in s.char_indices() {
        let escape = match c {
            '$' => true,
            '\n' => true,
            ' ' => escape_space,
            ':' => escape_colon,
            _ => false,
        };
        match output.as_mut() {
            Some(output) => {
                if escape {
                    output.push('$');
                }
                output.push(c);
            }
            None => {
                if escape {
                    let before = &s[..i];
                    let mut copied = before.to_owned();
                    copied.push('$');
                    copied.push(c);
                    output = Some(copied);
                }
            }
        }
    }
    match output {
        Some(output) => Cow::Owned(output),
        None => Cow::Borrowed(s),
    }
}

#[cfg(test)]
mod test_escape {
    use super::*;

    fn run_test_on_all(input: &str, output: &str) {
        assert_eq!(escape(input), output);
        assert_eq!(escape_path(input), output);
        assert_eq!(escape_build(input), output);
    }

    fn run_test_space(input: &str, output_no_escape_space: &str, output: &str) {
        assert_eq!(escape(input), output_no_escape_space);
        assert_eq!(escape_path(input), output);
        assert_eq!(escape_build(input), output);
    }

    fn run_test_colon_no_space(input: &str, output_no_escape_colon: &str, output: &str) {
        assert_eq!(escape(input), output_no_escape_colon);
        assert_eq!(escape_path(input), output_no_escape_colon);
        assert_eq!(escape_build(input), output);
    }

    #[test]
    fn test_empty() {
        run_test_on_all("", "");
    }

    #[test]
    fn test_no_escape() {
        run_test_on_all("foo", "foo");
        run_test_on_all("foo,bar", "foo,bar");
    }

    #[test]
    fn test_newline() {
        run_test_on_all("foo\nbar", "foo$\nbar");
        run_test_on_all("foo$\nbar", "foo$$$\nbar");
        run_test_on_all("\nfoobar\n", "$\nfoobar$\n");
    }

    #[test]
    fn test_dollar() {
        run_test_on_all("foo$$bar", "foo$$$$bar");
        run_test_on_all("foo$bar", "foo$$bar");
        run_test_on_all("$foobar$", "$$foobar$$");
    }

    #[test]
    fn test_space() {
        run_test_space("foo bar", "foo bar", "foo$ bar");
        run_test_space(" foo bar ", " foo bar ", "$ foo$ bar$ ");
        run_test_space("foo  bar", "foo  bar", "foo$ $ bar");
        run_test_space("foo\nb a r$baz", "foo$\nb a r$$baz", "foo$\nb$ a$ r$$baz");
    }

    #[test]
    fn test_colon() {
        run_test_colon_no_space("foo:bar", "foo:bar", "foo$:bar");
        run_test_colon_no_space("foo::bar", "foo::bar", "foo$:$:bar");
        run_test_colon_no_space("$foo:bar\n", "$$foo:bar$\n", "$$foo$:bar$\n");
    }

    #[test]
    fn test_all() {
        let input = "foo$\nb ar$$baz$:$qux";
        assert_eq!(escape(input), "foo$$$\nb ar$$$$baz$$:$$qux");
        assert_eq!(escape_path(input), "foo$$$\nb$ ar$$$$baz$$:$$qux");
        assert_eq!(escape_build(input), "foo$$$\nb$ ar$$$$baz$$$:$$qux");

        let input = "\u{4f60}he llo\u{597d}$\nb: ";
        assert_eq!(escape(input), "\u{4f60}he llo\u{597d}$$$\nb: ");
        assert_eq!(escape_path(input), "\u{4f60}he$ llo\u{597d}$$$\nb:$ ");
        assert_eq!(escape_build(input), "\u{4f60}he$ llo\u{597d}$$$\nb$:$ ");
    }
}

/// Macro for implementing variables function
macro_rules! implement_variables {
    ($($x:tt)*) => {
        /// Implementation for adding custom variables
        /// *(generated with `implement_variables` macro)*
        impl $($x)* {
            /// Set a custom variable in this block
            ///
            /// Note that built-in variables in ninja can usually be set using the other methods.
            pub fn variable<SName, SValue>(mut self, name: SName, value: SValue) -> Self
            where
                SName: AsRef<str>,
                SValue: AsRef<str>,
            {
                self.variables.push($crate::Variable::new(name, value));
                self
            }
        }
    }

}
pub(crate) use implement_variables;

/// Macro for adding a built-in variable
macro_rules! add_variable {
    ($variables:expr, $name:literal, $value:expr) => {{
        $variables.push($crate::Variable::new($name, $value));
    }};
}
pub(crate) use add_variable;
