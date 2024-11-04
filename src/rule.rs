//! Implementation of the `rule` keyword

use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::{Display, Formatter, Result};
use core::ops::Deref;

use crate::stmt::{Stmt, StmtRef};
use crate::util::{AddOnlyVec, Indented, RefCounted};
use crate::{Build, BuildRef, Ninja, Pool, ToArg, Variable, Variables};

/// A rule, as defined by the `rule` keyword
///
/// See <https://ninja-build.org/manual.html#_rules>
///
/// # Creating a rule
/// The most common way to create a rule is with the [`Ninja::rule`](crate::Ninja::rule) method,
/// which creates the rule, adds it to the ninja file, then returns a reference for configuration
/// ```rust
/// use ninja_writer::*;
///
/// let ninja = Ninja::new();
/// let rule = ninja.rule("cc", "gcc -MD -MF $out.d -c $in -o $out")
///     .depfile("$out.d")
///     .deps_gcc()
///     .description("Compiling $out");
/// rule.build(["foo.o"]).with(["foo.c"]);
///
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc -MD -MF $out.d -c $in -o $out
///   depfile = $out.d
///   deps = gcc
///   description = Compiling $out
///
/// build foo.o: cc foo.c
/// "###);
/// ```
///
/// # Using owned `Rule` type
/// If you prefer, you can create a rule with the [`Rule::new`](crate::Rule::new) method,
/// then add it to the ninja file with [`Rule::add_to`](crate::Rule::add_to)
/// to starting adding build edges. This may be useful if you want to configure a rule
/// in a function, since you don't need to deal with explicit lifetimes.
/// ```rust
/// use ninja_writer::*;
///
/// fn make_rule() -> Rule {
///     Rule::new("cc", "gcc -MD -MF $out.d -c $in -o $out")
///         .depfile("$out.d")
///         .deps_gcc()
///         .description("Compiling $out")
/// }
///
/// # fn main() {
/// // in the main function ...
/// let ninja = Ninja::new();
/// // `add_to` returns a reference, similar to `ninja.rule()`
/// let rule = make_rule().add_to(&ninja);
/// rule.build(["foo.o"]).with(["foo.c"]);
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc -MD -MF $out.d -c $in -o $out
///   depfile = $out.d
///   deps = gcc
///   description = Compiling $out
///
/// build foo.o: cc foo.c
/// "###);
/// # }
///
/// ```
///
/// # Thread safety
/// Enable the `thread-safe` flag to make the API thread-safe.
/// Here is an example of adding build edges to the same rule from
/// multiple threads. Note that the example will not compile without the `thread-safe` feature.
/// ```rust
/// # #[cfg(feature = "thread-safe")]
/// # {
/// use ninja_writer::*;
///
/// let ninja = Ninja::new();
///
/// // The type of `rule` below is `RuleRef`
/// // which implement `clone` to clone the underlying ref-counted pointer
/// let rule = ninja.rule("cc", "gcc -c $in -o $out");
/// let rule2 = rule.clone();
///
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc -c $in -o $out
/// "###);
/// assert_eq!(ninja.stmts.inner().len(), 1);
///
/// let t1 = std::thread::spawn(move || {
///     for _ in 0..100 {
///         rule.build(["foo1"]).with(["bar1"]);
///     }
/// });
///
/// let t2 = std::thread::spawn(move || {
///     for _ in 0..100 {
///         rule2.build(["foo2"]).with(["bar2"]);
///     }
/// });
///
/// t1.join().unwrap();
/// t2.join().unwrap();
/// assert_eq!(ninja.stmts.inner().len(), 201);
/// # }
/// ```
///
#[derive(Debug)]
pub struct Rule {
    /// The rule name as in `rule <name>`
    ///
    /// This is ref-counted so that it can be copied not-too-costly to build edges.
    pub name: RefCounted<String>,

    /// The list of variables, as an indented block
    ///
    /// See <https://ninja-build.org/manual.html#ref_rule>
    pub variables: AddOnlyVec<Variable>,
}

/// Trait for implementing variables for `rule` and `build`
pub trait RuleVariables: Variables {
    /// Set the depfile for this `rule` or `build` to explicitly support C/C++ header
    /// dependencies
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("cc", "gcc -c $in -o $out")
    ///     .depfile("$out.d");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///   depfile = $out.d
    /// "###);
    /// ```
    #[inline]
    fn depfile(self, depfile: impl ToArg) -> Self {
        self.variable("depfile", depfile)
    }

    /// Set `deps = gcc` for this `rule` or `build`
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("cc", "gcc -c $in -o $out")
    ///     .deps_gcc();
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///   deps = gcc
    /// "###);
    /// ```
    #[inline]
    fn deps_gcc(self) -> Self {
        self.variable("deps", "gcc")
    }

    /// Set `deps = msvc` for this rule (or build) and the `msvc_deps_prefix` variable
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("cl", "cl /c $in /Fo$out")
    ///     .deps_msvc_prefix("Note: including file: ");
    ///
    /// assert_eq!(ninja.to_string(), format!(r###"
    /// rule cl
    ///   command = cl /c $in /Fo$out
    ///   deps = msvc
    ///   msvc_deps_prefix = Note: including file: {}"###, "\n"));
    /// ```
    fn deps_msvc_prefix(self, msvc_deps_prefix: impl ToArg) -> Self {
        self.deps_msvc()
            .variable("msvc_deps_prefix", msvc_deps_prefix)
    }

    /// Set `deps = msvc` for this `rule` or `build` without `msvc_deps_prefix`
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("cl", "cl /c $in /Fo$out")
    ///     .deps_msvc();
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cl
    ///   command = cl /c $in /Fo$out
    ///   deps = msvc
    /// "###);
    /// ```
    #[inline]
    fn deps_msvc(self) -> Self {
        self.variable("deps", "msvc")
    }

    /// Set the description of the rule to be printed during the build
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("cc", "gcc -c $in -o $out")
    ///    .description("Compiling $out");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///   description = Compiling $out
    /// "###);
    /// ```
    #[inline]
    fn description(self, desc: impl ToArg) -> Self {
        self.variable("description", desc)
    }

    /// Indicate the rule is used to re-invoke the generator
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let configure = ninja.rule("configure", "cargo run --manifest-path ./configure/Cargo.toml -- $out")
    ///    .generator();
    ///
    /// configure.build(["build.ninja"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule configure
    ///   command = cargo run --manifest-path ./configure/Cargo.toml -- $out
    ///   generator = 1
    ///
    /// build build.ninja: configure
    /// "###);
    /// ```
    #[inline]
    fn generator(self) -> Self {
        self.variable("generator", "1")
    }

    /// Specify `restat = 1` for the `rule` or `build`
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("example", "...")
    ///     .restat();
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///   restat = 1
    /// "###);
    #[inline]
    fn restat(self) -> Self {
        self.variable("restat", "1")
    }

    /// Specify `rspfile` and `rspfile_content` variables for this `rule` or `build`
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("example", "...")
    ///     .rspfile("foo", "bar");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///   rspfile = foo
    ///   rspfile_content = bar
    /// "###);
    /// ```
    fn rspfile(self, rspfile: impl ToArg, rspfile_content: impl ToArg) -> Self {
        self.variable("rspfile", rspfile)
            .variable("rspfile_content", rspfile_content)
    }

    /// Set `pool = console` for this `rule` or `build`
    ///
    /// See <https://ninja-build.org/manual.html#_the_literal_console_literal_pool>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.rule("example", "...").pool_console();
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///   pool = console
    /// "###);
    /// ```
    fn pool_console(self) -> Self {
        self.variable("pool", "console")
    }

    /// Set the pool for this `rule` or `build`
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let pool = ninja.pool("expensive", 4);
    /// let rule = ninja.rule("cc", "gcc -c $in -o $out").pool(pool);
    /// rule.build(["foo.o"]).with(["foo.c"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// pool expensive
    ///   depth = 4
    ///
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///   pool = expensive
    ///
    /// build foo.o: cc foo.c
    /// "###);
    /// ```
    #[inline]
    fn pool(self, pool: impl AsRef<Pool>) -> Self {
        self.variable("pool", &pool.as_ref().name)
    }
}

/// Reference to a rule statement that can be used to create build edges
/// using this rule
#[derive(Debug, Clone)]
pub struct RuleRef(pub(crate) StmtRef);

impl Deref for RuleRef {
    type Target = Rule;
    fn deref(&self) -> &Self::Target {
        match self.0.deref().deref() {
            Stmt::Rule(r) => r,
            // safety: RuleRef is only constructable within this crate
            _ => unreachable!(),
        }
    }
}

impl AsRef<Rule> for RuleRef {
    fn as_ref(&self) -> &Rule {
        self.deref()
    }
}

impl RuleRef {
    /// Create a build edge using this rule and the explicit outputs, then add it to
    /// the ninja file provided.
    ///
    /// # Example
    /// See [`Rule`]
    pub fn build(&self, outputs: impl IntoIterator<Item = impl ToArg>) -> BuildRef {
        let build = Build::new(self.deref(), outputs);
        BuildRef(self.0.add(Stmt::Build(Box::new(build))))
    }
}

impl Rule {
    /// Create a new rule with the given name and command
    pub fn new(name: impl ToArg, command: impl ToArg) -> Self {
        let s = Self {
            name: RefCounted::new(name.to_arg()),
            variables: AddOnlyVec::new(),
        };
        s.variable("command", command)
    }

    /// Add the rule to a ninja file and return a [`RuleRef`] for further configuration
    pub fn add_to(self, ninja: &Ninja) -> RuleRef {
        RuleRef(ninja.add_stmt(Stmt::Rule(self)))
    }
}

impl Variables for Rule {
    #[inline]
    fn add_variable_internal(&self, v: Variable) {
        self.variables.add(v);
    }
}

impl RuleVariables for Rule {}

impl Variables for RuleRef {
    fn add_variable_internal(&self, v: Variable) {
        self.deref().add_variable_internal(v)
    }
}
impl RuleVariables for RuleRef {}

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "rule {}", self.name)?;
        for variable in self.variables.inner().iter() {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
