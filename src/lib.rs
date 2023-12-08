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

/// Top-level ninja statement
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// A Comment (`# <comment>`)
    Comment(String),

    /// A rule declaration
    ///
    /// See <https://ninja-build.org/manual.html#_rules>
    Rule(Rule),

    /// A build edge
    ///
    /// See <https://ninja-build.org/manual.html#_build_statements>
    Build(Build),

    /// A variable declaration
    ///
    /// See <https://ninja-build.org/manual.html#_variables>
    Variable(Variable),

    /// A default statement
    ///
    /// See <https://ninja-build.org/manual.html#_default_target_statements>
    Default(Vec<String>),
    /// A subninja statement
    ///
    /// See <https://ninja-build.org/manual.html#ref_scope>
    Subninja(String),

    /// An include statement (like subninja, but doesn't create a new scope)
    ///
    /// See <https://ninja-build.org/manual.html#ref_scope>
    Include(String),

    /// A pool declaration
    ///
    /// See <https://ninja-build.org/manual.html#ref_pool>
    Pool(Pool),
}

impl Stmt {
    pub fn ordinal(&self) -> usize {
        match self {
            Stmt::Comment(_) => 0,
            Stmt::Rule(_) => 1,
            Stmt::Build(_) => 2,
            Stmt::Variable(_) => 3,
            Stmt::Default(_) => 4,
            Stmt::Subninja(_) => 5,
            Stmt::Include(_) => 6,
            Stmt::Pool(_) => 7,
        }
    }
    pub fn is_same_type(&self, other: &Stmt) -> bool {
        self.ordinal() == other.ordinal()
    }
}

/// A rule, as defined by the `rule` keyword
///
/// See <https://ninja-build.org/manual.html#_rules>
///
/// # Special note for the `in` and `out` variables
/// Since these variables are usually not specified with an explicitly variable,
/// there's no function binding for it (also because `in` is a keyword in rust).
///
/// Instead, you have to specify it like:
/// ```rust
/// use ninja_writer::Rule;
///
/// let rule = Rule::new("foo", "foo $in $out");
/// rule.variable("in", "...");
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rule {
    /// The rule name as in `rule <name>`
    ///
    /// This is an [`Arc`] so that it can be copied not-too-costly to build edges.
    pub name: Arc<String>,

    /// The list of variables, as an indented block
    ///
    /// See <https://ninja-build.org/manual.html#ref_rule>
    pub variables: Vec<Variable>,
}
mod rule;
pub use rule::*;

/// A build edge, as defined by the `build` keyword
///
/// See <https://ninja-build.org/manual.html#_build_statements>
///
/// # Example
/// Since build edges are tied to rules, use [`RuleRef::build`](RuleRef::build) to create them.
/// ```rust
/// use ninja_writer::Ninja;
///
/// let mut ninja = Ninja::new();
/// let mut cc = ninja.rule("cc", "gcc $cflags -c $in -o $out");
/// cc.build(["foo.o"]).with(["foo.c"]);
///
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc $cflags -c $in -o $out
///
/// build foo.o: cc foo.c
/// "###);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Build {
    /// The rule name
    pub rule: Arc<String>,

    /// The list of outputs, as defined by `build <outputs>:`
    pub outputs: Vec<String>,

    /// The list of implicit outputs.
    ///
    /// See <https://ninja-build.org/manual.html#ref_outputs>
    pub implicit_outputs: Vec<String>,

    /// The list of dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub dependencies: Vec<String>,

    /// The list of implicit dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub implicit_dependencies: Vec<String>,

    /// The list of order-only dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub order_only_dependencies: Vec<String>,

    /// The list of validations.
    ///
    /// See <https://ninja-build.org/manual.html#validations>
    pub validations: Vec<String>,

    /// The list of variables, as an indented block
    pub variables: Vec<Variable>,
}
mod build;
pub use build::*;

/// A pool, as defined by the `pool` keyword
///
/// See <https://ninja-build.org/manual.html#ref_pool>
///
/// # Example
/// ```rust
/// use ninja_writer::{Ninja, Pool};
///
/// let mut ninja = Ninja::new();
/// let mut expensive = ninja.pool("expensive", 4)
///     .variable("foo", "bar");
///
/// let compile = expensive.rule("compile", "gcc $cflags -c $in -o $out");
///
/// assert_eq!(ninja.to_string(), r###"
/// pool expensive
///   depth = 4
///   foo = bar
///
/// rule compile
///   command = gcc $cflags -c $in -o $out
///   pool = expensive
/// "###);
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pool {
    /// If this pool is built-in to ninja, such as `console`
    pub built_in: bool,

    /// Name of the pool
    pub name: String,
    /// The list of variables, as an indented block
    ///
    /// Currently the only useful variable is `depth`
    pub variables: Vec<Variable>,
}
mod pool;
pub use pool::*;

/// A variable declaration (`name = value`)
///
/// See <https://ninja-build.org/manual.html#_variables>
///
/// # Escaping
/// Escaping must be done when constructing the variable. No escaping is done when serializing.
/// ```rust
/// use ninja_writer::Variable;
///
/// let var = Variable::new("foo", "I have a $ in me");
///
/// assert_eq!(var.to_string(), "foo = I have a $ in me");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// The name of the variable
    pub name: String,
    /// The value of the variable
    pub value: String,
}

impl Variable {
    /// Create a new variable declaration
    pub fn new<SName, SValue>(name: SName, value: SValue) -> Self
    where
        SName: AsRef<str>,
        SValue: AsRef<str>,
    {
        Self {
            name: name.as_ref().to_owned(),
            value: value.as_ref().to_owned(),
        }
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} = {}", self.name, self.value)
    }
}

mod util;
pub use util::{escape, escape_build, escape_path};
