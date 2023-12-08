//! Implementation of the `pool` keyword

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::vec;
use core::fmt::{Display, Formatter, Result};
use core::ops::{Deref, DerefMut};

use crate::util::{implement_variables, Indented};
use crate::{Ninja, Pool, Rule, RuleRef, Stmt, Variable};

/// A structure returned by the `pool` method of [`Ninja`], so that `rule` statements
/// are automatically added.
pub struct PoolRef<'ninja> {
    ninja: &'ninja mut Ninja,
    statement_index: usize,
}

impl<'ninja> PoolRef<'ninja> {
    pub fn from(ninja: &'ninja mut Ninja, statement_index: usize) -> Self {
        Self {
            ninja,
            statement_index,
        }
    }

    /// Create a rule that will run in this pool, with the given name and command.
    ///
    /// The rule is automatically added to the ninja file.
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// let mut expensive_pool = ninja.pool("expensive", 1);
    /// let mut rule = expensive_pool.rule("cat", "cat $in > $out");
    /// rule.build(["foo"]).with(["bar"]);
    ///
    /// let mut rule2 = expensive_pool.rule("meow", "cat $in > $out");
    /// rule2.build(["foo2"]).with(["bar2"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// pool expensive
    ///   depth = 1
    ///
    /// rule cat
    ///   command = cat $in > $out
    ///   pool = expensive
    ///
    /// build foo: cat bar
    ///
    /// rule meow
    ///   command = cat $in > $out
    ///   pool = expensive
    ///
    /// build foo2: meow bar2
    /// "###);
    pub fn rule<SName, SCommand>(&mut self, name: SName, command: SCommand) -> RuleRef<'_>
    where
        SName: AsRef<str>,
        SCommand: AsRef<str>,
    {
        let rule = Rule::new(name, command).variable("pool", self.name.clone());
        self.ninja.add_rule(rule)
    }
}

implement_variables!(<'a> PoolRef<'a>);

impl<'ninja> AsRef<Pool> for PoolRef<'ninja> {
    fn as_ref(&self) -> &Pool {
        match self.ninja.statements.get(self.statement_index).unwrap() {
            Stmt::Pool(pool) => pool,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> AsMut<Pool> for PoolRef<'ninja> {
    fn as_mut(&mut self) -> &mut Pool {
        match self.ninja.statements.get_mut(self.statement_index).unwrap() {
            Stmt::Pool(pool) => pool,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> Deref for PoolRef<'ninja> {
    type Target = Pool;

    fn deref(&self) -> &Self::Target {
        match self.ninja.statements.get(self.statement_index).unwrap() {
            Stmt::Pool(pool) => pool,
            _ => unreachable!(),
        }
    }
}

impl<'ninja> DerefMut for PoolRef<'ninja> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.ninja.statements.get_mut(self.statement_index).unwrap() {
            Stmt::Pool(pool) => pool,
            _ => unreachable!(),
        }
    }
}

impl Pool {
    /// Create a pool with a given name and depth
    pub fn new<SName, SDepth>(name: SName, depth: SDepth) -> Self
    where
        SName: AsRef<str>,
        SDepth: Display,
    {
        Self {
            built_in: false,
            name: name.as_ref().to_owned(),
            variables: vec![Variable::new("depth", format!("{depth}"))],
        }
    }

    /// Add the pool to a ninja file
    #[inline]
    pub fn add_to(self, ninja: &mut Ninja) -> PoolRef<'_> {
        ninja.add_pool(self)
    }
}

implement_variables!(Pool);

impl Display for Pool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.built_in {
            return Ok(());
        }
        writeln!(f, "pool {}", self.name)?;
        for variable in &self.variables {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
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
        let pool = Pool::new("foo", 1);
        assert_eq!(pool.to_string(), "pool foo\n  depth = 1\n");
    }

    #[test]
    fn test_variable() {
        let pool = Pool::new("foo", 42).variable("foov", "z");
        assert_eq!(pool.to_string(), "pool foo\n  depth = 42\n  foov = z\n");
    }
}
