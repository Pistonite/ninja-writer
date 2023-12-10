//! Implementation of top-level stuff

use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use alloc::rc::Rc;
use core::fmt::{Display, Formatter, Result};

use crate::{Build, BuildRef, Pool, PoolRef, Rule, RuleRef, Stmt, Variable, StmtList, StmtVec, StmtVecSync, };

/// The main entry point for writing a ninja file.
///
/// # Examples
/// See the [crate-level documentation](crate)
///
/// # Thread safety
/// `Ninja::new` creates an instance that is not thread-safe.
/// `NinjaSync::new` creates an instance that is safe to add statements
/// from multiple threads.
///
/// See the [crate-level documentation](crate) for more examples.
#[derive(Debug, Clone, PartialEq)]
pub struct NinjaInternal<TList, TRc> where TList: StmtList<TRc=TRc> {
    /// The list of statements
    pub stmts: TList,

    /// The built-in phony rule,
    pub phony: Rule,
}
pub type Ninja = NinjaInternal<StmtVec, Rc<Stmt>>;
pub type NinjaSync = NinjaInternal<StmtVecSync, Arc<Stmt>>;

impl<TList, TRc> NinjaInternal<TList, TRc> where TList: StmtList<TRc=TRc> {
    /// Create a blank ninja file
    pub fn new() -> Self {
        Self {
            phony: Rule::new("phony", ""),
            stmts: TList::default(),
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
    pub fn rule<SName, SCommand>(&self, name: SName, command: SCommand) -> RuleRef<'_, TList, TRc>
    where
        SName: AsRef<str>,
        SCommand: AsRef<str>,
    {
        Rule::new(name, command).add_to(self)
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
    pub fn phony<SOutputIter, SOutput>(&mut self, outputs: SOutputIter) -> BuildRef<'_, TList, TRc>
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        let build = Build::new(&self.phony, outputs);
        BuildRef {
            inner: self.stmts.add(Stmt::Build(build))
        }
    }

    /// Create a new [`Pool`] with the name and depth and add it to this ninja file.
    /// Returns a reference of the pool for configuration, and for adding rules and builds to the
    /// pool.
    #[inline]
    pub fn pool<SName, SDepth>(&self, name: SName, depth: SDepth) -> PoolRef<'_, TList, TRc>
    where
        SName: AsRef<str>,
        SDepth: Display,
    {
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
    pub fn comment<SComment>(&self, comment: SComment) -> &Self
    where
        SComment: AsRef<str>,
    {
        self.stmts.add(Stmt::Comment(comment.as_ref().to_owned()));
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
    pub fn variable<SName, SValue>(&self, name: SName, value: SValue) -> &Self
    where
        SName: AsRef<str>,
        SValue: AsRef<str>,
    {
        self.stmts.add(Stmt::Variable(Variable::new(name, value)));
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
    pub fn defaults<SOutputIter, SOutput>(&self, outputs: SOutputIter) -> &Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        self.stmts.add(Stmt::Default(
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
    pub fn subninja<SPath>(&self, path: SPath) -> &Self
    where
        SPath: AsRef<str>,
    {
        self.stmts.add(Stmt::Subninja(path.as_ref().to_owned()));
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
    pub fn include<SPath>(&self, path: SPath) -> &Self
    where
        SPath: AsRef<str>,
    {
        self.stmts.add(Stmt::Include(path.as_ref().to_owned()));
        self
    }
}

impl Display for Ninja {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.stmts.fmt(f)
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
