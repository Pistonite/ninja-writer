//! Statment implementations

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Deref;

use crate::util::{AddOnlyVec, RefCounted};
use crate::{Build, Pool, Rule, Variable};

/// A top-level ninja statement
#[derive(Debug)]
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
    Build(Box<Build>),

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
    /// Get the ordinal for this statement type
    pub fn ordinal(&self) -> usize {
        match self {
            Self::Comment(_) => 0,
            Self::Rule(_) => 1,
            Self::Build(_) => 2,
            Self::Variable(_) => 3,
            Self::Default(_) => 4,
            Self::Subninja(_) => 5,
            Self::Include(_) => 6,
            Self::Pool(_) => 7,
        }
    }

    pub fn is_same_type(&self, other: &Self) -> bool {
        self.ordinal() == other.ordinal()
    }
}

/// A reference to a statement in a list that can be used to get the statement,
/// as well as add new statements to the list.
#[derive(Debug)]
pub struct StmtRef {
    /// The list this statement is in
    pub(crate) list: RefCounted<AddOnlyVec<RefCounted<Stmt>>>,
    /// The statement
    pub(crate) stmt: RefCounted<Stmt>,
}
impl StmtRef {
    /// Add a new statment to the list this statement is in
    pub fn add(&self, stmt: Stmt) -> StmtRef {
        StmtRef {
            stmt: self.list.add_rc(stmt),
            list: RefCounted::clone(&self.list),
        }
    }
}
impl Clone for StmtRef {
    fn clone(&self) -> Self {
        Self {
            list: RefCounted::clone(&self.list),
            stmt: RefCounted::clone(&self.stmt),
        }
    }
}
impl Deref for StmtRef {
    type Target = RefCounted<Stmt>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.stmt
    }
}

impl AsRef<Stmt> for StmtRef {
    #[inline]
    fn as_ref(&self) -> &Stmt {
        self.deref()
    }
}
