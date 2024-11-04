//! # ninja-writer
//! ![Build Badge](https://img.shields.io/github/actions/workflow/status/Pistonite/ninja-writer/rust.yml)
//! ![Version Badge](https://img.shields.io/crates/v/ninja-writer)
//! ![License Badge](https://img.shields.io/github/license/Pistonite/ninja-writer)
//! ![Issue Badge](https://img.shields.io/github/issues/Pistonite/ninja-writer)
//!
//! Library for writing [ninja](https://ninja-build.org) build files with a focus on
//! ergonomics and simplicity.
//!
//! Since Rust requires a trait to be in scope to use,
//! it is recommended to import `*` from the crate, so that all the traits
//! are in scope.
//! ```rust
//! use ninja_writer::*;
//! ```
//!
//! ## Why another?
//! I found existing libraries poorly documented, and I want to explore
//! and learn the syntax for ninja myself.
//!
//! ## Example
//! The [`Ninja`] struct is the main entry point for writing a ninja file.
//! It is used to make top-level declarations, such as variables and rules.
//! It implements [`Display`](core::fmt::Display) so that it can be converted to a string, written to a file, etc.
//!
//! Here's a complete example of writing a ninja file that builds a simple C program.
//! See [`Ninja`] for all the methods available.
//!
//! ```rust
//! use ninja_writer::*;
//!
//! // Create writer
//! let ninja = Ninja::new();
//! // Create a variable
//! ninja.variable("cflags", "-Wall -Wextra -Werror");
//! // Create the cc rule
//! let cc = ninja.rule("cc", "gcc -MD -MF $depfile $cflags -c $in -o $out")
//!     .description("Compiling $out")
//!     .depfile("$out.d")
//!     .deps_gcc();
//! // Create the ld rule
//! let ld = ninja.rule("ld", "gcc -o $out $in")
//!     .description("Linking $out");
//!
//! // Create build edges using the rules
//! cc.build(["foo.o"]).with(["foo.c"]);
//! cc.build(["bar.o"]).with(["bar.c"])
//!     .variable("cflags", "-Wall -DDEBUG");
//!
//! ld.build(["app"]).with(["foo.o", "bar.o"]);
//!
//! ninja.defaults(["app"]);
//!
//! let ninja_file: String = ninja.to_string();
//! assert_eq!(ninja_file, r###"
//! cflags = -Wall -Wextra -Werror
//!
//! rule cc
//!   command = gcc -MD -MF $depfile $cflags -c $in -o $out
//!   description = Compiling $out
//!   depfile = $out.d
//!   deps = gcc
//!
//! rule ld
//!   command = gcc -o $out $in
//!   description = Linking $out
//!
//! build foo.o: cc foo.c
//! build bar.o: cc bar.c
//!   cflags = -Wall -DDEBUG
//! build app: ld foo.o bar.o
//!
//! default app
//! "###);
//! ```
//!
//! ## Encoding
//! Because `.to_string()` is used to get the output, All inputs/outputs are expected to be UTF-8
//! encoded. Utilities like [`ToArg`] will panic for `std` types if the input is not valid UTF-8.
//!
//! ## Args and lists
//! All functions take implementation of [`ToArg`] as parameters.
//! This trait is implemented for common Rust types like [`Path`](std::path::Path) and
//! [`String`].
//!
//! For functions that take a list of arguments (such as [`build`](RuleRef::build)),
//! the types of the elements in the slice must be the same due to Rust's type system restrictions.
//! ```compile_fail
//! // This won't compile
//! let args = [1, "bar"];
//! ```
//! The [`args`] macro is provided to workaround this limitation.
//!
//!
//! ## `std` feature
//! You can disable the `std` feature to make the library `no_std` compatible. I don't know why you
//! want to do that, but it's here just in case.
//!
//! ## Thread safety
//! By default, the API is not thread-safe. However, you can enable the `thread-safe` feature,
//! which uses `Arc` and `RwLock` to ensure thread safety.
//!
//! Here's an example of using 2 threads to configure 200 rules.
//! (It's highly theoretical. [`Rule`] has a more realistic example
//! where multiple threads configure build edges on the same rule)
//! ```rust
//! # #[cfg(feature = "thread-safe")]
//! # {
//! use ninja_writer::*;
//! use std::sync::Arc;
//!
//! let ninja = Arc::new(Ninja::new());
//! let ninja1 = Arc::clone(&ninja);
//! let ninja2 = Arc::clone(&ninja);
//! let t1 = std::thread::spawn(move || {
//!     for i in 0..100 {
//!         ninja1.rule("example", "...");
//!     }
//! });
//! let t2 = std::thread::spawn(move || {
//!     for i in 0..100 {
//!         ninja2.rule("example", "...");
//!     }
//! });
//! t1.join().unwrap();
//! t2.join().unwrap();
//!
//! assert_eq!(ninja.stmts.inner().len(), 200);
//! # }
//! ```
//! The example won't compile unless you enable the `thread-safe` feature.
//!
//! ## Escaping
//! There is an [`escape`] function that can be used to escape strings
//! according to [the behavior](https://ninja-build.org/manual.html#ref_lexer) of ninja.
//! ```rust
//! use ninja_writer::escape;
//!
//! assert_eq!(escape("foo"), "foo");
//! assert_eq!(escape("$foo"), "$$foo");
//! assert_eq!(escape("foo bar"), "foo bar");
//! assert_eq!(escape("foo: bar"), "foo: bar");
//! ```
//! Since it's only necessary to escape spaces in list of paths, you can use [`escape_path`] to do that:
//! ```rust
//! use ninja_writer::escape_path;
//! assert_eq!(escape_path("foo bar"), "foo$ bar");
//! ```
//! Similarly, [`escape_build`] can be used to escape both spaces and `:`s, for
//! specifying outputs.
//! ```rust
//! use ninja_writer::escape_build;
//! assert_eq!(escape_build("foo: bar"), "foo$:$ bar");
//! ```
//!
//! ## Duplicated variables
//! Duplicates are not checked, since ninja allows it.
//! ```rust
//! use ninja_writer::Ninja;
//!
//! let mut ninja = Ninja::new();
//! ninja.variable("foo", "bar");
//! ninja.variable("foo", "bar again");
//!
//! assert_eq!(ninja.to_string(), r###"
//! foo = bar
//! foo = bar again
//! "###);
//! ```
//!
//! ## Order of statements
//! The order of statements is preserved. Ninja's variables are expanded
//! immediately except for in rules, so the order of statements does matter.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[doc(hidden)]
pub mod arg;
#[doc(hidden)]
pub mod build;
#[doc(hidden)]
pub mod ninja;
#[doc(hidden)]
pub mod pool;
#[doc(hidden)]
pub mod rule;
#[doc(hidden)]
pub mod stmt;
#[doc(hidden)]
pub mod util;
#[doc(hidden)]
pub mod variable;

// Re-exports
pub use arg::ToArg;
pub use build::{Build, BuildRef, BuildVariables};
pub use ninja::Ninja;
pub use pool::{Pool, PoolRef};
pub use rule::{Rule, RuleRef, RuleVariables};
pub use util::{escape, escape_build, escape_path};
pub use variable::{Variable, Variables};
