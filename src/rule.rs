//! Implementation of the `rule` keyword

use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use alloc::vec;
use core::fmt::{Display, Formatter, Result};
use core::ops::{Deref, DerefMut};

use crate::util::{implement_variables, Indented};
use crate::{Build, BuildRef, Ninja, Rule, Stmt, Variable};

/// A structure returned by the `rule` method of [`Ninja`], so that `build` statements
/// are automatically added.
pub struct RuleRef<'ninja> {
    ninja: &'ninja mut Ninja,
    statement_index: usize,
}

impl<'ninja> RuleRef<'ninja> {
    pub fn from(ninja: &'ninja mut Ninja, statement_index: usize) -> Self {
        Self {
            ninja,
            statement_index,
        }
    }

    /// Create a build edge with the given outputs.
    ///
    /// The build edge is automatically added to the ninja file.
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// let mut rule = ninja.rule("cat", "cat $in > $out");
    /// rule.build(["foo"]).with(["bar"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cat
    ///   command = cat $in > $out
    ///
    /// build foo: cat bar
    /// "###);
    pub fn build<SOutputIter, SOutput>(&mut self, outputs: SOutputIter) -> BuildRef<'_>
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        let build = Build::new(self, outputs);
        self.ninja.add_build(build)
    }
}

impl<'ninja> AsRef<Rule> for RuleRef<'ninja> {
    fn as_ref(&self) -> &Rule {
        match self.ninja.statements.get(self.statement_index).unwrap() {
            Stmt::Rule(rule) => rule,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> AsMut<Rule> for RuleRef<'ninja> {
    fn as_mut(&mut self) -> &mut Rule {
        match self.ninja.statements.get_mut(self.statement_index).unwrap() {
            Stmt::Rule(rule) => rule,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> Deref for RuleRef<'ninja> {
    type Target = Rule;

    fn deref(&self) -> &Self::Target {
        match self.ninja.statements.get(self.statement_index).unwrap() {
            Stmt::Rule(rule) => rule,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> DerefMut for RuleRef<'ninja> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.ninja.statements.get_mut(self.statement_index).unwrap() {
            Stmt::Rule(rule) => rule,
            _ => unreachable!(),
        }
    }
}

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
pub(crate) use implement_rule_variables;

implement_rule_variables!(<'a> RuleRef<'a>);
implement_variables!(<'a> RuleRef<'a>);

impl Rule {
    /// Create a new rule with the given name and command
    pub fn new<SName, SCommand>(name: SName, command: SCommand) -> Self
    where
        SName: AsRef<str>,
        SCommand: AsRef<str>,
    {
        Self {
            name: Arc::new(name.as_ref().to_owned()),
            variables: vec![Variable::new("command", command)],
        }
    }

    /// Create a build edge using this rule and the explicit outputs, then add it to
    /// the ninja file provided.
    ///
    /// This should only be used if you are constructing a Rule explicitly with `Rule::new()`.
    /// Note that the rule itself is not automatically added to the ninja file.
    ///
    /// # Example
    /// Note that in the example below, the build is added to the ninja file first.
    /// ```rust
    /// use ninja_writer::{Ninja, Rule};
    ///
    /// let mut ninja = Ninja::new();
    /// let rule = Rule::new("cat", "cat $in > $out");
    /// rule.build(&mut ninja, ["foo"]).with(["bar"]);
    /// rule.add_to(&mut ninja);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// build foo: cat bar
    ///
    /// rule cat
    ///   command = cat $in > $out
    /// "###);
    /// ```
    /// Usually you would either `ninja.add_rule(rule)` before configuring the build edges,
    /// or use `ninja.rule` to create the rule. In either of these cases, you don't need to
    /// specify the ninja file instance when calling `build`, because it is done through
    /// [`RuleRef`]
    /// ```rust
    /// use ninja_writer::{Ninja, Rule};
    ///
    /// let mut ninja = Ninja::new();
    /// let mut rule1 = ninja.rule("foo1", "...");
    /// rule1.build(["foo"]).with(["bar"]);
    ///
    /// let rule2 = Rule::new("foo2", "...");
    /// let mut rule2 = rule2.add_to(&mut ninja); // ninja.add_rule(rule2) would also work
    /// rule2.build(["fiz"]).with(["bar"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule foo1
    ///   command = ...
    ///
    /// build foo: foo1 bar
    ///
    /// rule foo2
    ///   command = ...
    ///
    /// build fiz: foo2 bar
    /// "###);
    /// ```
    pub fn build<'ninja, SOutputIter, SOutput>(
        &self,
        ninja: &'ninja mut Ninja,
        outputs: SOutputIter,
    ) -> BuildRef<'ninja>
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        let build = Build::new(self, outputs);
        ninja.add_build(build)
    }

    /// Add the rule to a ninja file and return a [`RuleRef`] for further configuration
    #[inline]
    pub fn add_to(self, ninja: &mut Ninja) -> RuleRef<'_> {
        ninja.add_rule(self)
    }
}
implement_rule_variables!(Rule);
implement_variables!(Rule);

impl Display for Rule {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "rule {}", self.name)?;
        for variable in &self.variables {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
