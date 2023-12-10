//! # ninja-writer
//!
//! Library for writing [ninja](https://ninja-build.org) build files, similar to ninja-syntax or ninja-build-syntax.
//!
//! ## Why another?
//! I found existing libraries poorly documented, and I want to explore
//! and learn the syntax for ninja myself.
//!
//! ## Example
//! Here's a simple but complete example of writing a ninja file that builds a simple C program.
//!
//! ```rust
//! use ninja_writer::Ninja;
//!
//! // Create writer
//! let mut ninja = Ninja::new();
//! // Create a variable
//! ninja.variable("cflags", "-Wall -Wextra -Werror");
//! // Create a rule
//! let mut cc = ninja.rule("cc", "gcc -MD -MF $depfile $cflags -c $in -o $out")
//!     .description("CC $out")
//!     .depfile("$out.d")
//!     .deps_gcc();
//! // Create build edges using the rule
//! cc.build(["foo.o"]).with(["foo.c"]);
//! cc.build(["bar.o"]).with(["bar.c"])
//!     .variable("cflags", "-Wall -DDEBUG");
//!
//! let mut link = ninja.rule("link", "gcc -o $out $in")
//!    .description("LINK $out");
//! link.build(["app"]).with(["foo.o", "bar.o"]);
//!
//! ninja.defaults(["app"]);
//!
//! let ninja_file: String = ninja.to_string();
//! assert_eq!(ninja_file, r###"
//! cflags = -Wall -Wextra -Werror
//!
//! rule cc
//!   command = gcc -MD -MF $depfile $cflags -c $in -o $out
//!   description = CC $out
//!   depfile = $out.d
//!   deps = gcc
//!
//! build foo.o: cc foo.c
//! build bar.o: cc bar.c
//!   cflags = -Wall -DDEBUG
//!
//! rule link
//!   command = gcc -o $out $in
//!   description = LINK $out
//!
//! build app: link foo.o bar.o
//!
//! default app
//! "###);
//! ```
//!
//! ## `std` feature
//! You can disable the `std` feature to make the library `no_std` compatible. I don't know why you
//! want to do that, but it's here just in case.
//!
//! ## The `Ninja` struct
//! The [`Ninja`] struct is the main entry point for writing a ninja file.
//! It is used to make top-level declarations, such as variables and rules.
//! It implements [`Display`] so that it can be converted to a string, written to a file, etc.
//!
//! ## Rules
//! Rules can be created with the [`rule`](Ninja::rule) function from `Ninja`.
//! Rules created this way are automatically added to the ninja file.
//! ```rust
//! use ninja_writer::Ninja;
//!
//! let mut ninja = Ninja::new();
//! // Create a rule
//! let cc = ninja.rule("cc", "gcc $cflags -c $in -o $out");
//! assert_eq!(ninja.to_string(), r###"
//! rule cc
//!   command = gcc $cflags -c $in -o $out
//! "###);
//! ```
//!
//! You can also create owned [`Rule`]s with [`Rule::new`](Rule::new).
//! Then use [`add_rule`](Ninja::add_rule) or [`add_to`](Rule::add_to) to add them to the ninja file.
//! ```rust
//! use ninja_writer::{Ninja, Rule};
//!
//! let mut ninja = Ninja::new();
//! let cc: Rule = Rule::new("cc", "gcc $cflags -c $in -o $out");
//!
//! let cc = cc.add_to(&mut ninja);
//! // ... use cc.build to configure build edges
//!
//! let link: Rule = Rule::new("link", "gcc -o $out $in");
//! let link = ninja.add_rule(link);
//! // ... use link.build to configure build edges
//!
//! assert_eq!(ninja.to_string(), r###"
//! rule cc
//!   command = gcc $cflags -c $in -o $out
//!
//! rule link
//!   command = gcc -o $out $in
//! "###);
//! ```
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
//! ## Arg lists
//! For functions that take a list of arguments (such as [`build`](RuleRef::build)),
//! the types of the elements in the slice must be the same due to Rust's type system restrictions.
//! ```no_compile
//! // This won't compile
//! let foo = "foo".to_string();
//! let args = [foo, "bar"];
//! ```
//! You can either call `.as_ref()` on each element to convert them to `&str`s,
//! or define a simple macro to do this for you to avoid sprinkling `.as_ref()` everywhere.
//! ```rust
//! macro_rules! refs {
//!     ($($x:expr),* $(,)?) => {
//!          vec![$($x.as_ref()),*]
//!     }
//! }
//! ```
//! This can be useful if you have custom types that implement `AsRef<str>`.
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
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result};

/// The main entry point for writing a ninja file.
#[derive(Debug, Clone, PartialEq)]
pub struct Ninja {
    /// The list of statements
    pub statements: Vec<Stmt>,

    /// The built-in phony rule,
    pub phony: Rule,
}

mod ninja;
pub use ninja::*;

mod stmt;
pub use stmt::*;


mod rule;
pub use rule::*;

mod build;
pub use build::*;

mod pool;
pub use pool::*;


mod variable;
pub use variable::*;

mod util;
pub use util::{escape, escape_build, escape_path};
