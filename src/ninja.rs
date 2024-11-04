//! Implementation of top-level stuff

use alloc::boxed::Box;
use core::fmt::{Display, Formatter, Result};

use crate::stmt::{Stmt, StmtRef};
use crate::util::{AddOnlyVec, RefCounted};
use crate::{Build, BuildRef, Pool, PoolRef, Rule, RuleRef, ToArg, Variable};

/// The main entry point for writing a ninja file.
///
/// # Examples
/// See the [crate-level documentation](crate)
#[derive(Debug)]
pub struct Ninja {
    /// The list of statements
    pub stmts: RefCounted<AddOnlyVec<RefCounted<Stmt>>>,

    /// The built-in phony rule,
    pub phony: Rule,
}

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
            stmts: Default::default(),
        }
    }

    /// Create a new rule with the given name and command and add it to this ninja file.
    ///
    /// The returned [`RuleRef`] can be used to configure the rule and build edges
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("cc", "gcc -c $in -o $out");
    /// rule.build(["foo.o"]).with(["foo.c"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule cc
    ///   command = gcc -c $in -o $out
    ///
    /// build foo.o: cc foo.c
    /// "###);
    #[inline]
    pub fn rule(&self, name: impl ToArg, command: impl ToArg) -> RuleRef {
        Rule::new(name, command).add_to(self)
    }

    /// Add a new build edge with the `phony` rule, used for aliasing
    ///
    /// See <https://ninja-build.org/manual.html#_the_literal_phony_literal_rule>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.phony(["all"]).with(["foo.o", "bar.o"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// build all: phony foo.o bar.o
    /// "###);
    /// ```
    pub fn phony(&self, outputs: impl IntoIterator<Item = impl ToArg>) -> BuildRef {
        let build = Build::new(&self.phony, outputs);
        BuildRef(self.add_stmt(Stmt::Build(Box::new(build))))
    }

    /// Create a new [`Pool`] with the name and depth and add it to this ninja file.
    /// Returns a reference of the pool for configuration, and for adding rules and builds to the
    /// pool.
    #[inline]
    pub fn pool(&self, name: impl ToArg, depth: usize) -> PoolRef {
        Pool::new(name, depth).add_to(self)
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
    /// assert_eq!(ninja.to_string(), r###"
    /// ## This is a comment
    /// "###);
    /// ```
    pub fn comment(&self, comment: impl ToArg) -> &Self {
        self.stmts.add_rc(Stmt::Comment(comment.to_arg()));
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
    pub fn variable(&self, name: impl ToArg, value: impl ToArg) -> &Self {
        self.stmts
            .add_rc(Stmt::Variable(Variable::new(name, value)));
        self
    }

    /// Add a default statement
    ///
    /// See <https://ninja-build.org/manual.html#_default_target_statements>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// ninja.defaults(["foo", "bar"]);
    /// ninja.defaults(["baz"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// default foo bar
    /// default baz
    /// "###);
    /// ```
    pub fn defaults(&self, outputs: impl IntoIterator<Item = impl ToArg>) -> &Self {
        self.stmts.add_rc(Stmt::Default(
            outputs.into_iter().map(|s| s.to_arg()).collect(),
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
    pub fn subninja(&self, path: impl ToArg) -> &Self {
        self.stmts.add_rc(Stmt::Subninja(path.to_arg()));
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
    pub fn include(&self, path: impl ToArg) -> &Self {
        self.stmts.add_rc(Stmt::Include(path.to_arg()));
        self
    }

    /// Internal function to add a statement
    pub(crate) fn add_stmt(&self, stmt: Stmt) -> StmtRef {
        StmtRef {
            stmt: self.stmts.add_rc(stmt),
            list: RefCounted::clone(&self.stmts),
        }
    }
}

impl Display for Ninja {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let list = &self.stmts.inner();
        if list.is_empty() {
            return Ok(());
        }
        let mut last = 0;
        for stmt in list.iter() {
            let stmt = stmt.as_ref();
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
        let ninja = Ninja::new();
        assert_eq!(ninja.to_string(), "");
    }

    // doc tests should give enough coverage
}
