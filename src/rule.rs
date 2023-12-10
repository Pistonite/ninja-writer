//! Implementation of the `rule` keyword

use alloc::borrow::ToOwned;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::cell::RefCell;
use core::fmt::{Display, Formatter, Result};
use core::ops::{Deref, DerefMut};

use crate::util::Indented;
use crate::{Build, Stmt, Variable, StmtRef, Variables, StmtVec, BuildRef, StmtList, StmtVecSync, Pool, NinjaInternal};

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
///     .deps_gcc();
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
///     let rule = Rule::new("cc", "gcc -MD -MF $out.d -c $in -o $out");
///     rule.depfile("$out.d")
///         .deps_gcc()
///         .description("Compiling $out");
///     rule
/// }
///
/// fn main() {
///     let ninja = Ninja::new();
///     // `add_to` returns a reference, similar to `ninja.rule()`
///     let rule = make_rule().add_to(&ninja);
///     rule.build(["foo.o"]).with(["foo.c"]);
/// }
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
/// # Thread safety
/// Use [`NinjaSync`](crate::NinjaSync) instead of `Ninja` to 
/// ensure thread safety when adding build edges to the same rule from
/// multiple threads.
/// ```rust
/// use ninja_writer::*;
///
/// let ninja = NinjaSync::new();
///
/// // The type of `rule` below is `RuleRef<StmtVecSync, Arc<Stmt>>`
/// // which implement `clone` to clone the underlying ref-counted pointer
/// let rule = ninja.rule("cc", "gcc -c $in -o $out");
/// let rule2 = rule.clone();
///
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc -c $in -o $out
/// "###);
/// assert_eq!(ninja.stmts.len(), 1);
///
/// let t1 = std::thread::spawn(move || {
///     for _ in 0..100 {
///         rule.build(["foo1"]).with(["bar1"]);
///     }
/// };
///
/// let t2 = std::thread::spawn(move || {
///     for _ in 0..100 {
///         rule2.build(["foo2"]).with(["bar2"]);
///     }
/// };
///
/// t1.join().unwrap();
/// t2.join().unwrap();
/// assert_eq!(ninja.stmts.len(), 201);
/// ```
///
/// Note that this only ensures thread safety for **adding build edges**,
/// not setting variables! Do not set variables on the same rule from multiple threads.
/// ```should_panic
/// use ninja_writer::*;
///
/// let ninja = NinjaSync::new();
/// let rule = ninja.rule("cc", "gcc -c $in -o $out");
/// let rule2 = rule.clone();
///
/// let t1 = std::thread::spawn(move || {
///     // simulate a condition where t1 has variables mutably borrowed
///     let x = rule.variables.borrow_mut();
///     std::thread::sleep(std::time::Duration::from_millis(1000));
///     drop(x);
/// };
/// let t2 = std::thread::spawn(move || {
///     for _ in 0..100 {
///         rule2.variable("a", "b");
///     }
/// };
///
/// t1.join().unwrap();
/// t2.join().unwrap(); // should panic
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
    pub variables: RefCell<Vec<Variable>>,
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
    fn depfile<SDepfile>(&self, depfile: SDepfile) -> &Self
where
        SDepfile: AsRef<str>,
    {
        self.variable("depfile", depfile)
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
    fn pool<TPool>(&self, pool: TPool) -> &Self
    where TPool: Deref<Target=Pool> {
        self.variable("pool", &pool.name)
    }
}

/// Reference to a rule statement that can be used to create build edges
/// using this rule
#[derive(Debug, Clone)]
pub struct RuleRef<'a, TList, TRc> where TList: StmtList<TRc=TRc> {
    inner: StmtRef<'a, TList, TRc> 
}
impl<'a, TList, TRc> Deref for RuleRef<'a, TList, TRc>
where TList: StmtList<TRc=TRc>, TRc: Deref<Target=Stmt> {
    type Target = Rule;
    fn deref(&self) -> &Self::Target {
        match self.inner.deref().deref() {
            Stmt::Rule(r) => r,
            _ => panic!("Expected rule statement"),
        }
    }
}
impl<'a, TList, TRc> Variables for RuleRef<'a, TList, TRc>
where TList: StmtList<TRc=TRc>, TRc: Deref<Target=Stmt> {
    fn add_variable_internal(&self, v: Variable) {
        self.deref().variables.borrow_mut().push(v);
    }
}
impl<'a, TList, TRc> RuleVariables for RuleRef<'a, TList, TRc>
where TList: StmtList<TRc=TRc>, TRc: Deref<Target=Stmt> {
}
impl<'a, TList, TRc> RuleRef<'a, TList, TRc> where TList: StmtList<TRc=TRc>
, TRc: Deref<Target=Stmt>
{
    /// Create a build edge using this rule and the explicit outputs, then add it to
    /// the ninja file provided.
    ///
    /// # Example
    /// See [`Rule`]
    pub fn build<SOutputIter, SOutput>(
        &self,
        outputs: SOutputIter,
    ) -> BuildRef<'_, TList, TRc>
where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        let build = Build::new(self.deref(), outputs);
        BuildRef {
            inner: self.inner.add(Stmt::Build(build))
        }
    }
}



// /// A structure returned by the `rule` method of [`Ninja`], so that `build` statements
// /// are automatically added.
// pub struct RuleRef<'ninja> {
//     ninja: &'ninja mut Ninja,
//     statement_index: usize,
// }
//
// impl<'ninja> RuleRef<'ninja> {
//     pub fn from(ninja: &'ninja mut Ninja, statement_index: usize) -> Self {
//         Self {
//             ninja,
//             statement_index,
//         }
//     }
//
//     /// Create a build edge with the given outputs.
//     ///
//     /// The build edge is automatically added to the ninja file.
//     ///
//     /// # Example
//     /// ```rust
//     /// use ninja_writer::Ninja;
//     ///
//     /// let mut ninja = Ninja::new();
//     /// let mut rule = ninja.rule("cat", "cat $in > $out");
//     /// rule.build(["foo"]).with(["bar"]);
//     ///
//     /// assert_eq!(ninja.to_string(), r###"
//     /// rule cat
//     ///   command = cat $in > $out
//     ///
//     /// build foo: cat bar
//     /// "###);
//     pub fn build<SOutputIter, SOutput>(&mut self, outputs: SOutputIter) -> BuildRef<'_>
//     where
//         SOutputIter: IntoIterator<Item = SOutput>,
//         SOutput: AsRef<str>,
//     {
//         let build = Build::new(self, outputs);
//         self.ninja.add_build(build)
//     }
// }
//
// impl<'ninja> AsRef<Rule> for RuleRef<'ninja> {
//     fn as_ref(&self) -> &Rule {
//         match self.ninja.statements.get(self.statement_index).unwrap() {
//             Stmt::Rule(rule) => rule,
//             _ => unreachable!(),
//         }
//     }
// }
//
// impl<'ninja> AsMut<Rule> for RuleRef<'ninja> {
//     fn as_mut(&mut self) -> &mut Rule {
//         match self.ninja.statements.get_mut(self.statement_index).unwrap() {
//             Stmt::Rule(rule) => rule,
//             _ => unreachable!(),
//         }
//     }
// }
//
// impl<'ninja> Deref for RuleRef<'ninja> {
//     type Target = Rule;
//
//     fn deref(&self) -> &Self::Target {
//         match self.ninja.statements.get(self.statement_index).unwrap() {
//             Stmt::Rule(rule) => rule,
//             _ => unreachable!(),
//         }
//     }
// }
//
// impl<'ninja> DerefMut for RuleRef<'ninja> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         match self.ninja.statements.get_mut(self.statement_index).unwrap() {
//             Stmt::Rule(rule) => rule,
//             _ => unreachable!(),
//         }
//     }
// }


/// Implement all the built-in variables for a rule or build
macro_rules! implement_rule_variables {
    ($($x:tt)*) => {
        /// Impementation for variables that are shared between `rule` and `build`
        /// *(generated with `implement_rule_variables` macro)*
        impl $($x)* {

            /// Set the depfile for this `rule` or `build` to explicitly support C/C++ header
            /// dependencies
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("cc", "gcc -c $in -o $out")
            ///     .depfile("$out.d");
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule cc
            ///   command = gcc -c $in -o $out
            ///   depfile = $out.d
            /// "###);
            /// ```
            pub fn depfile<SDepfile>(mut self, depfile: SDepfile) -> Self
            where
                SDepfile: AsRef<str>,
            {
                $crate::util::add_variable!(self.variables, "depfile", depfile);
                self
            }

            /// Set `deps = gcc` for this `rule` or `build`
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("cc", "gcc -c $in -o $out")
            ///     .deps_gcc();
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule cc
            ///   command = gcc -c $in -o $out
            ///   deps = gcc
            /// "###);
            /// ```
            pub fn deps_gcc(mut self) -> Self {
                $crate::util::add_variable!(self.variables, "deps", "gcc");
                self
            }

            /// Set `deps = msvc` for this rule (or build) and the `msvc_deps_prefix` variable
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("cl", "cl /c $in /Fo$out")
            ///     .deps_msvc_prefix("Note: including file: ");
            ///
            /// assert_eq!(ninja.to_string(), format!(r###"
            /// rule cl
            ///   command = cl /c $in /Fo$out
            ///   deps = msvc
            ///   msvc_deps_prefix = Note: including file: {}"###, "\n"));
            /// ```
            pub fn deps_msvc_prefix<SMsvcDepsPrefix>(self, msvc_deps_prefix: SMsvcDepsPrefix) -> Self
            where
                SMsvcDepsPrefix: AsRef<str>,
            {
                let mut x = self.deps_msvc();
                $crate::util::add_variable!(x.variables, "msvc_deps_prefix", msvc_deps_prefix);
                x
            }

            /// Set `deps = msvc` for this `rule` or `build` without `msvc_deps_prefix`
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("cl", "cl /c $in /Fo$out")
            ///     .deps_msvc();
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule cl
            ///   command = cl /c $in /Fo$out
            ///   deps = msvc
            /// "###);
            /// ```
            pub fn deps_msvc(mut self) -> Self {
                $crate::util::add_variable!(self.variables, "deps", "msvc");
                self
            }

            /// Set the description of the rule to be printed during the build
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("cc", "gcc -c $in -o $out")
            ///    .description("Compiling $out");
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule cc
            ///   command = gcc -c $in -o $out
            ///   description = Compiling $out
            /// "###);
            /// ```
            pub fn description<SDesc>(mut self, desc: SDesc) -> Self
            where
                SDesc: AsRef<str>,
            {
                $crate::util::add_variable!(self.variables, "description", desc);
                self
            }

            /// Indicate the rule is used to re-invoke the generator
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// let mut configure = ninja.rule("configure", "cargo run --manifest-path ./configure/Cargo.toml -- $out")
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
            pub fn generator(mut self) -> Self {
                $crate::util::add_variable!(self.variables, "generator", "1");
                self
            }

            /// Specify the `in_newline` variable for the `rule` or `build`
            ///
            /// See <https://ninja-build.org/manual.html#ref_rule>
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("example", "...")
            ///    .in_newline("foo");
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule example
            ///   command = ...
            ///   in_newline = foo
            /// "###);
            /// ```
            pub fn in_newline<SIn>(mut self, in_newline: SIn) -> Self
            where
                SIn: AsRef<str>,
            {
                $crate::util::add_variable!(self.variables, "in_newline", in_newline);
                self
            }

            /// Specify `restat = 1` for the `rule` or `build`
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("example", "...")
            ///     .restat();
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule example
            ///   command = ...
            ///   restat = 1
            /// "###);
            pub fn restat(mut self) -> Self {
                $crate::util::add_variable!(self.variables, "restat", "1");
                self
            }

            /// Specify `rspfile` and `rspfile_content` variables for this `rule` or `build`
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
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
            pub fn rspfile<SRspfile, SRspfileContent>(mut self, rspfile: SRspfile, rspfile_content: SRspfileContent) -> Self
            where
                SRspfile: AsRef<str>,
                SRspfileContent: AsRef<str>,
            {
                $crate::util::add_variable!(self.variables, "rspfile", rspfile);
                $crate::util::add_variable!(self.variables, "rspfile_content", rspfile_content);
                self
            }

            /// Set `pool = console` for this `rule` or `build`
            ///
            /// See <https://ninja-build.org/manual.html#_the_literal_console_literal_pool>
            /// **Note: If you need to specify a custom pool, see [Ninja::pool](`crate::Ninja::pool`)**
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// ninja.rule("example", "...").pool_console();
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule example
            ///   command = ...
            ///   pool = console
            /// "###);
            /// ```
            pub fn pool_console(mut self) -> Self {
                $crate::util::add_variable!(self.variables, "pool", "console");
                self
            }

        }

    };
}
// pub(crate) use implement_rule_variables;

// implement_rule_variables!(<'a> RuleRef<'a>);
// implement_variables!(<'a> RuleRef<'a>);

impl Rule {
    /// Create a new rule with the given name and command
    pub fn new<SName, SCommand>(name: SName, command: SCommand) -> Self
where
        SName: AsRef<str>,
        SCommand: AsRef<str>,
    {
        let s = Self {
            name: Arc::new(name.as_ref().to_owned()),
            variables: Default::default(),
        };
        s.variable("command", command);
        s
    }


    /// Add the rule to a ninja file and return a [`RuleRef`] for further configuration
    pub fn add_to<TList, TRc>(self, ninja: &NinjaInternal<TList, TRc>) -> RuleRef<'_, TList, TRc>
    where TList: StmtList<TRc=TRc> {
        RuleRef{
            inner: ninja.stmts.add(Stmt::Rule(self))
        }
    }
}

impl Variables for Rule {
    fn add_variable_internal(&self, v: Variable) {
        self.variables.borrow_mut().push(v);
    }
}

impl RuleVariables for Rule {}

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "rule {}", self.name)?;
        for variable in self.variables.borrow().iter() {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
