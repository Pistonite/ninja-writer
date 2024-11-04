//! Helper trait to convert Rust types to arguments
use alloc::string::String;

/// Convert something to an argument
///
/// This is a helper trait to convert Rust types to arguments:
/// - Numbers and Booleans
/// - [`String`] and [`&str`][str]
/// - [`OsString`][std::ffi::OsString] and [`&OsStr`][std::ffi::OsStr]
/// - [`PathBuf`][std::path::PathBuf] and [`&Path`][std::path::Path]
/// - `u8` slices and vectors
///
/// # Panic
/// Conversion from `OsString`, `Path` and byte slices will panic if they are not valid UTF-8.
/// These implementations are only available with the `std` feature.
///
/// # Mixing types
/// If you need to mix types within the same list, you can use the [`args!`](crate::args) macro.
/// It's currently only available in `std` due to its limited usefullness in `no_std` environments.
/// ```rust
/// # #[cfg(feature = "std")]
/// # {
/// use std::path::Path;
///
/// use ninja_writer::*;
///
/// let ninja = Ninja::new();
/// let cc = ninja.rule("ld", "gcc -o $out in");
/// let a_o: String = "a.o".to_string();
/// cc.build(["foo"]).with(args!["b.o", a_o, Path::new("c.o")]);
///
/// assert_eq!(ninja.to_string(), r###"
/// rule ld
///   command = gcc -o $out in
///
/// build foo: ld b.o a.o c.o
/// "###);
/// # }
/// ```
pub trait ToArg {
    /// Convert the type to an argument
    ///
    /// This trait is implemented for many types in the std library
    /// for convenience.
    ///
    /// # Panic
    /// This will panic if the type is not valid UTF-8.
    /// Only implementations enabled by the `std` feature will panic.
    fn to_arg(self) -> String;
}

/// Convert a mixed list of arguments types to a list of strings
///
/// See examples in [`ToArg`].
#[cfg(feature = "std")]
#[macro_export]
macro_rules! args {
    ($($x:expr),* $(,)?) => {{
        let v: ::std::vec::Vec<::std::string::String>= ::std::vec![$($x.to_arg()),*];
        v
    }}
}

#[cfg(feature = "std")]
#[cfg(test)]
mod args_tests {
    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(args![], Vec::<String>::new());
    }

    #[test]
    fn test_single() {
        assert_eq!(args![1], vec!["1"]);
    }

    #[test]
    fn test_single_trailing() {
        assert_eq!(args![1,], vec!["1"]);
    }

    #[test]
    fn test_multiple() {
        assert_eq!(args![1, "foo", true], vec!["1", "foo", "true"]);
    }

    #[test]
    fn test_multiple_trailing() {
        assert_eq!(args![1, "foo", true,], vec!["1", "foo", "true"]);
    }
}

#[cfg(test)]
fn accepts_to_arg(val: impl ToArg) -> String {
    val.to_arg()
}

macro_rules! impl_with {
    (to_owned for $($ty:ty),*) => {
        $(
        impl ToArg for $ty {
            fn to_arg(self) -> String {
                self.to_owned()
            }
        }
        )*
    };

    (to_string for $($ty:ty),*) => {
        $(
        impl ToArg for $ty {
            fn to_arg(self) -> String {
                self.to_string()
            }
        }
        )*
    };

    (as_os_str for $($ty:ty),*) => {
        $(
        impl ToArg for $ty {
            fn to_arg(self) -> String {
                self.as_os_str().to_arg()
            }
        }
        )*
    };
}

macro_rules! test_case {
    ($ty:ty, $name:ident, $val:expr) => {
        #[cfg(test)]
        mod $name {
            use super::*;

            #[test]
            fn to_arg() {
                let val: $ty = { $val };
                let _ = accepts_to_arg(val);
            }
        }
    };

    ($ty:ty, $name:ident, $val:expr, $expected:expr) => {
        #[cfg(test)]
        mod $name {
            use super::*;

            #[test]
            fn to_arg() {
                let val: $ty = { $val };
                assert_eq!(accepts_to_arg(val), $expected);
            }
        }
    };
}

#[rustfmt::skip]
mod impls {
    use super::*;

    use alloc::borrow::ToOwned;
    use alloc::string::ToString;

    impl ToArg for String {
        fn to_arg(self) -> String { self }
    }
    test_case!(String, string, String::from("foo"));

    impl_with!(to_owned for &str, &String); 
    test_case!(&str, str_ref, "foo");
    test_case!(&String, string_ref, &String::from("foo"));

    impl_with!(to_string for 
        i8, i16, i32, i64, i128, isize, 
        u8, u16, u32, u64, u128, usize,
        bool
    );
    test_case!(i8, i8_, 1i8, "1");
    test_case!(i16, i16_, 1i16, "1");
    test_case!(i32, i32_, 1i32, "1");
    test_case!(i64, i64_, 1i64, "1");
    test_case!(i128, i128_, 1i128, "1");
    test_case!(isize, isize_, 1isize, "1");
    test_case!(u8, u8_, 1u8, "1");
    test_case!(u16, u16_, 1u16, "1");
    test_case!(u32, u32_, 1u32, "1");
    test_case!(u64, u64_, 1u64, "1");
    test_case!(u128, u128_, 1u128, "1");
    test_case!(usize, usize_, 1usize, "1");
    test_case!(bool, bool_true, true, "true");
    test_case!(bool, bool_false, false, "false");

    #[cfg(feature = "std")]
    mod std_impls {
        use std::{ffi::{OsStr, OsString}, path::{Path, PathBuf}};

        use super::*;

        impl ToArg for &[u8] {
            fn to_arg(self) -> String {
                String::from_utf8(self.to_vec()).unwrap()
            }
        }
        test_case!(&[u8], u8_slice_ref, b"foo", "foo");

        impl ToArg for Vec<u8> {
            fn to_arg(self) -> String {
                String::from_utf8(self).unwrap()
            }
        }
        test_case!(Vec<u8>, u8_vec, vec![ 0xe4,0xbd,0xa0,0xe5,0xa5,0xbd ], "\u{4f60}\u{597d}");

        impl ToArg for &Vec<u8> {
            fn to_arg(self) -> String {
                String::from_utf8(self.clone()).unwrap()
            }
        }
        test_case!(&Vec<u8>, u8_vec_ref, &vec![ 0xe4,0xbd,0xa0,0xe5,0xa5,0xbd ], "\u{4f60}\u{597d}");

        impl ToArg for &OsStr {
            fn to_arg(self) -> String {
                self.to_str().unwrap().to_owned()
            }
        }
        test_case!(&OsStr, os_str_ref, OsStr::new("foo"), "foo");

        impl ToArg for OsString {
            fn to_arg(self) -> String {
                self.into_string().unwrap()
            }
        }
        test_case!(OsString, os_string, OsString::from("foo"), "foo");

        impl_with!(as_os_str for &Path, &OsString, PathBuf, &PathBuf);
        test_case!(&OsString, os_string_ref, &OsString::from("foo"), "foo");
        test_case!(&Path, path_ref, Path::new("foo"), "foo");
        test_case!(PathBuf, path_buf, "/foo/".into(), "/foo/");
        test_case!(&PathBuf, path_buf_ref, &PathBuf::from("/foo/"), "/foo/");
    }
    
    
    
}
