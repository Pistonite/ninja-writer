//! Implementation of the `pool` keyword

use alloc::borrow::ToOwned;
use alloc::format;
use core::cell::RefCell;
use core::fmt::{Display, Formatter, Result};
use core::ops::Deref;

use crate::{StmtRef, StmtList, Variables, NinjaInternal};
use crate::util::Indented;
use crate::{Stmt, Variable};

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
/// # Thread safety
/// Calling `pool` on a `NinjaSync` from multiple threads is safe.
/// However, configuring variables on a pool is not thread-safe (even with
/// [`NinjaSync`](crate::NinjaSync)). 
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pool {
    /// Name of the pool
    pub name: String,
    /// The list of variables, as an indented block
    ///
    /// Currently the only useful variable is `depth`
    pub variables: RefCell<Vec<Variable>>,
}

/// Reference to a pool statement
#[derive(Debug)]
pub struct PoolRef<'a, TList, TRc>
where TList: StmtList<TRc=TRc> {
        inner:StmtRef<'a, TList, TRc>
    }


impl<'a, TList, TRc> Deref for PoolRef<'a, TList, TRc> where TList: StmtList<TRc=TRc>
, TRc: Deref<Target=Stmt> {
    type Target = Pool;
    fn deref(&self) -> &Self::Target {
        match self.inner.deref().deref() {
            Stmt::Pool(p) => p,
            _ => panic!("Expected pool statement"),
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
        let x = Self {
            name: name.as_ref().to_owned(),
            variables: Default::default(),
        };
        x.variable("depth", format!("{depth}"));
        x
    }

    /// Add the pool to a ninja file
    #[inline]
    pub fn add_to<TList, TRc>(self, ninja: &NinjaInternal<TList, TRc>) -> PoolRef<'_, TList, TRc>
    where TList: StmtList<TRc=TRc> {
        PoolRef {
            inner: ninja.stmts.add(Stmt::Pool(self)),
        }
    }
}

impl Variables for Pool {
    fn add_variable_internal(&self, v: Variable) {
        self.variables.borrow_mut().push(v);
    }
}

impl Display for Pool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "pool {}", self.name)?;
        for variable in self.variables.borrow().iter() {
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
        let pool = Pool::new("foo", 42)
            ;
        pool.variable("foov", "z");
        assert_eq!(pool.to_string(), "pool foo\n  depth = 42\n  foov = z\n");
    }
}
