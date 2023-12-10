//! Statment implementations

use core::cell::RefCell;
use core::fmt::{Display, Formatter, Result};
use core::ops::Deref;
use std::sync::RwLock;

use alloc::rc::Rc;
use alloc::sync::Arc;

use crate::{Build, Pool, Rule, Variable};

/// A top-level ninja statement
#[derive(Debug, Clone, PartialEq)]
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
    Build(Build),

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
            Stmt::Comment(_) => 0,
            Stmt::Rule(_) => 1,
            Stmt::Build(_) => 2,
            Stmt::Variable(_) => 3,
            Stmt::Default(_) => 4,
            Stmt::Subninja(_) => 5,
            Stmt::Include(_) => 6,
            Stmt::Pool(_) => 7,
        }
    }

    pub fn is_same_type(&self, other: &Stmt) -> bool {
        self.ordinal() == other.ordinal()
    }
}

/// Trait to support multiple implementations of a statement list
///
/// Used for a thread-safe implementation and non-thread-safe one.
pub trait StmtList: Sized + Default + Display {
    /// Ref-counting smart pointer type
    type TRc: Deref<Target = Stmt> + Clone;

    /// Add a statment and return a mutable reference to it
    fn add(&self, stmt: Stmt) -> StmtRef<Self, Self::TRc>;

    /// Get the number of statements in the list
    ///
    /// This is used to validate some test cases, particularly
    /// in multi-thread scenarios.
    fn len(&self) -> usize;
}

/// A reference to a statement in a list that can be used to get the statement,
/// as well as add new statements to the list.
#[derive(Debug)]
pub struct StmtRef<'a, TList, TRc> where TList: StmtList<TRc=TRc> {
    list: &'a TList,
    stmt: TRc,
}
impl<'a, TList, TRc> StmtRef<'a, TList, TRc> where TList: StmtList<TRc=TRc> {
    /// Add a new statment to the list this statement is in
    pub fn add(&self, stmt: Stmt) -> StmtRef<'a, TList, TRc> {
        self.list.add(stmt)
    }
}

impl<'a, TList, TRc> Clone for StmtRef<'a, TList, TRc> where 
TList: StmtList<TRc=TRc>,
TRc: Clone {
    fn clone(&self) -> Self {
        Self {
            list: self.list,
            stmt: self.stmt.clone(),
        }
    }

}

impl<'a, TList, TRc> AsRef<TRc> for StmtRef<'a, TList, TRc> where TList: StmtList<TRc=TRc>
{
    #[inline]
    fn as_ref(&self) -> &TRc {
        &self.stmt
    }
}
impl<'a, TList, TRc> Deref for StmtRef<'a, TList, TRc> where TList: StmtList<TRc=TRc> 
{
    type Target = TRc;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.stmt
    }
}

/// Non-thread-safe implementation of a statement list
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StmtVec {
    stmts: RefCell<Vec<Rc<Stmt>>>,
}

impl StmtList for StmtVec {
    type TRc = Rc<Stmt>;
    fn add(&self, stmt: Stmt) -> StmtRef<Self, Self::TRc> {
        let rc = Rc::new(stmt);
        let mut stmts = self.stmts.borrow_mut();
        stmts.push(Rc::clone(&rc));
        StmtRef {
            list: self,
            stmt: rc,
        }
    }

    fn len(&self) -> usize {
        self.stmts.borrow().len()
    }

}

impl Display for StmtVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write_stmt_list(f, &self.stmts.borrow())
    }
}

/// Thread-safe implementation of a statement list
#[derive(Debug, Default)]
pub struct StmtVecSync {
    stmts: RwLock<Vec<Arc<Stmt>>>,
}
impl StmtList for StmtVecSync {
    type TRc = Arc<Stmt>;
    fn add(&self, stmt: Stmt) -> StmtRef<Self, Self::TRc> {
        let rc = Arc::new(stmt);
        let mut stmts = self.stmts.write().unwrap();
        stmts.push(Arc::clone(&rc));
        StmtRef {
            list: self,
            stmt: rc,
        }
    }

    fn len(&self) -> usize {
        self.stmts.read().unwrap().len()
    }
}

impl Display for StmtVecSync {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write_stmt_list(f, &self.stmts.read().unwrap())
    }
}

/// Helper function to format a list of statements
fn write_stmt_list<TStmt>(f: &mut Formatter<'_>, list: &[TStmt]) -> Result
where
    TStmt: AsRef<Stmt>,
{
    if list.is_empty() {
        return Ok(());
    }
    let mut last = 0;
    for stmt in list {
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
