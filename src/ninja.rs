//! Implementation of top-level stuff

use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result};

use crate::{Build, BuildRef, Ninja, Pool, PoolRef, Rule, RuleRef, Stmt, Variable};

impl Default for Ninja {
    fn default() -> Self {
        Self::new()
    }
}

impl Ninja {
    /// Create a blank ninja file
    pub fn new() -> Self {
        Self {
            phony: Rule::new("phony", ""),
            statements: Vec::new(),
        }
    }

    /// Create a new rule with the given name and command and add it to this ninja file.
    ///
    /// The returned [`RuleRef`] can be used to configure the rule and build edges
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// let mut rule = ninja.rule("cc", "gcc -c $in -o $out");
    /// rule.build(["foo.o"]).with(["foo.c"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///
    /// build foo.o: cc foo.c
    /// "###);
    #[inline]
    pub fn rule<SName, SCommand>(&mut self, name: SName, command: SCommand) -> RuleRef
    where
        SName: AsRef<str>,
        SCommand: AsRef<str>,
    {
        self.add_rule(Rule::new(name, command))
    }

    /// Add a new build edge with the `phony` rule, used for aliasing
    ///
    /// See <https://ninja-build.org/manual.html#_the_literal_phony_literal_rule>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.phony(["all"]).with(["foo.o", "bar.o"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// build all: phony foo.o bar.o
    /// "###);
    /// ```
    pub fn phony<SOutputIter, SOutput>(&mut self, outputs: SOutputIter) -> BuildRef<'_>
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        self.add_build(Build::new(&self.phony, outputs))
    }

    /// Add a rule and return a reference of it for configuration
    ///
    /// # Note
    /// Use this when you have created a rule with [`Rule::new`](Rule::new),
    /// and want to add it to this ninja file.
    ///
    /// The returned [`RuleRef`] can be used to configure the rule and build edges
    /// using the rule. Build edges created with this ref are automatically
    /// added to this ninja file.
    pub fn add_rule(&mut self, rule: Rule) -> RuleRef {
        self.statements.push(Stmt::Rule(rule));
        RuleRef::from(self, self.statements.len() - 1)
    }

    /// Add a build edge
    ///
    /// Usually you will not use this method directly, but instead,
    /// use [`rule`](Self::rule) to create a rule
    /// and configure build edges using the returned [`RuleRef`].
    pub fn add_build(&mut self, build: Build) -> BuildRef<'_> {
        self.statements.push(Stmt::Build(build));
        match self.statements.last_mut().unwrap() {
            Stmt::Build(build) => BuildRef(build),
            _ => unreachable!(),
        }
    }

    /// Create a new [`Pool`] with the name and depth and add it to this ninja file.
    /// Returns a reference of the pool for configuration.
    #[inline]
    pub fn pool<SName, SDepth>(&mut self, name: SName, depth: SDepth) -> PoolRef<'_>
    where
        SName: AsRef<str>,
        SDepth: Display,
    {
        self.add_pool(Pool::new(name, depth))
    }

    /// Add a pool and return a reference of it for configuration
    ///
    /// Usually you will use [`pool`](Self::pool) instead of this method.
    pub fn add_pool(&mut self, pool: Pool) -> PoolRef<'_> {
        self.statements.push(Stmt::Pool(pool));
        PoolRef::from(self, self.statements.len() - 1)
    }

    /// Add a comment
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.comment("This is a comment");
    ///
    /// assert_eq!(ninja.to_string(), "\n# This is a comment\n");
    /// ```
    pub fn comment<SComment>(&mut self, comment: SComment) -> &mut Self
    where
        SComment: AsRef<str>,
    {
        self.statements
            .push(Stmt::Comment(comment.as_ref().to_owned()));
        self
    }

    /// Add a top-level variable
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.variable("foo", "bar");
    /// ninja.variable("baz", "qux $bar");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// foo = bar
    /// baz = qux $bar
    /// "###);
    /// ```
    pub fn variable<SName, SValue>(&mut self, name: SName, value: SValue) -> &mut Self
    where
        SName: AsRef<str>,
        SValue: AsRef<str>,
    {
        self.statements
            .push(Stmt::Variable(Variable::new(name, value)));
        self
    }

    /// Add a default statement
    ///
    /// See <https://ninja-build.org/manual.html#_default_target_statements>
    ///
    /// **Note that [`default`](Self::default) is a different function that is used to create
    /// Ninja.**
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.defaults(["foo", "bar"]);
    /// ninja.defaults(["baz"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// default foo bar
    /// default baz
    /// "###);
    /// ```
    pub fn defaults<SOutputIter, SOutput>(&mut self, outputs: SOutputIter) -> &mut Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        self.statements.push(Stmt::Default(
            outputs.into_iter().map(|s| s.as_ref().to_owned()).collect(),
        ));
        self
    }

    /// Add a subninja statement
    ///
    /// See <https://ninja-build.org/manual.html#ref_scope>
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.subninja("foo.ninja");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// subninja foo.ninja
    /// "###);
    /// ```
    pub fn subninja<SPath>(&mut self, path: SPath) -> &mut Self
    where
        SPath: AsRef<str>,
    {
        self.statements
            .push(Stmt::Subninja(path.as_ref().to_owned()));
        self
    }

    /// Add an include statement.
    ///
    /// The difference between `include` and [`subninja`](Self::subninja) is that
    /// `include` brings the variables into the current scope, much like `#include` in C.
    ///
    /// See <https://ninja-build.org/manual.html#ref_scope>
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// ninja.include("foo.ninja");
    /// ninja.include("bar.ninja");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// include foo.ninja
    /// include bar.ninja
    /// "###);
    /// ```
    pub fn include<SPath>(&mut self, path: SPath) -> &mut Self
    where
        SPath: AsRef<str>,
    {
        self.statements
            .push(Stmt::Include(path.as_ref().to_owned()));
        self
    }
}

impl Display for Ninja {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.statements.is_empty() {
            return Ok(());
        }
        let mut last = 0;
        for stmt in &self.statements {
            // have a blank line between statement types and between rules
            let next = stmt.ordinal() + 1;
            if matches!(stmt, Stmt::Rule(_)) || next != last {
                writeln!(f)?;
            }
            last = next;

            match stmt {
                Stmt::Rule(rule) => rule.fmt(f)?,
                Stmt::Build(build) => build.fmt(f)?,
                Stmt::Pool(pool) => pool.fmt(f)?,
                Stmt::Comment(comment) => writeln!(f, "# {}", comment)?,
                Stmt::Variable(variable) => {
                    variable.fmt(f)?;
                    writeln!(f)?;
                }
                Stmt::Default(outputs) => {
                    write!(f, "default")?;
                    for output in outputs {
                        write!(f, " {}", output)?;
                    }
                    writeln!(f)?;
                }
                Stmt::Subninja(path) => writeln!(f, "subninja {}", path)?,
                Stmt::Include(path) => writeln!(f, "include {}", path)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_default() {
        let ninja = Ninja::default();
        assert_eq!(ninja.to_string(), "");
    }

    // doc tests should give enough coverage
}
