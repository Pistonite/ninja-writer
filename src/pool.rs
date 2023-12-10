//! Implementation of the `pool` keyword

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use core::fmt::{Display, Formatter, Result};
use core::ops::Deref;

use crate::stmt::{Stmt, StmtRef};
use crate::util::{AddOnlyVec, Indented};
use crate::{Ninja, Variable, Variables};

/// A pool, as defined by the `pool` keyword
///
/// See <https://ninja-build.org/manual.html#ref_pool>
///
/// # Example
/// ```rust
/// use ninja_writer::*;
///
/// let ninja = Ninja::new();
/// let expensive = ninja.pool("expensive", 4)
///     .variable("foo", "bar");
///
/// let compile = ninja.rule("compile", "gcc $cflags -c $in -o $out").pool(&expensive);
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
/// ```
#[derive(Debug)]
pub struct Pool {
    /// Name of the pool
    pub name: String,
    /// The list of variables, as an indented block
    ///
    /// Currently the only useful variable is `depth`
    pub variables: AddOnlyVec<Variable>,
}

/// Reference to a pool statement
#[derive(Debug, Clone)]
pub struct PoolRef(pub(crate) StmtRef);

impl Deref for PoolRef {
    type Target = Pool;
    fn deref(&self) -> &Self::Target {
        match self.0.deref().deref() {
            Stmt::Pool(p) => p,
            _ => panic!("Expected pool statement"),
        }
    }
}

impl AsRef<Pool> for PoolRef {
    fn as_ref(&self) -> &Pool {
        self.deref()
    }
}

impl Pool {
    /// Create a pool with a given name and depth
    pub fn new<SName, SDepth>(name: SName, depth: SDepth) -> Self
    where
        SName: AsRef<str>,
        SDepth: Display,
    {
        let x = Self {
            name: name.as_ref().to_owned(),
            variables: AddOnlyVec::new(),
        };
        x.variable("depth", format!("{depth}"))
    }

    /// Add the pool to a ninja file
    #[inline]
    pub fn add_to(self, ninja: &Ninja) -> PoolRef {
        PoolRef(ninja.add_stmt(Stmt::Pool(self)))
    }
}

impl Variables for Pool {
    fn add_variable_internal(&self, v: Variable) {
        self.variables.add(v);
    }
}

impl Variables for PoolRef {
    fn add_variable_internal(&self, v: Variable) {
        self.deref().add_variable_internal(v);
    }
}

impl Display for Pool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "pool {}", self.name)?;
        for variable in self.variables.inner().iter() {
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
