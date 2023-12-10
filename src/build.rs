//! Implementation of the `build` keyword

use alloc::borrow::ToOwned;
use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::{Display, Formatter, Result};
use core::ops::{Deref, DerefMut};

use crate::util::Indented;
use crate::{Rule, Variables, Variable, StmtRef, Stmt, StmtVec, StmtVecSync, StmtList, RuleVariables};

/// A build edge, as defined by the `build` keyword
///
/// See <https://ninja-build.org/manual.html#_build_statements>
///
/// # Example
/// Since build edges are tied to rules, use [`RuleRef::build`](RuleRef::build) to create them.
/// ```rust
/// use ninja_writer::Ninja;
///
/// let mut ninja = Ninja::new();
/// let mut cc = ninja.rule("cc", "gcc $cflags -c $in -o $out");
/// cc.build(["foo.o"]).with(["foo.c"]);
///
/// assert_eq!(ninja.to_string(), r###"
/// rule cc
///   command = gcc $cflags -c $in -o $out
///
/// build foo.o: cc foo.c
/// "###);
///
/// ```
/// # Thread safety
/// Configuring variables/dependencies/outputs on a build edge is not thread-safe (even with
/// [`NinjaSync`](crate::NinjaSync)). Do not add variables to the same build edge on multiple
/// threads.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Build {
    /// The rule name
    pub rule: Arc<String>,

    /// The list of outputs, as defined by `build <outputs>:`
    pub outputs: RefCell<Vec<String>>,

    /// The list of implicit outputs.
    ///
    /// See <https://ninja-build.org/manual.html#ref_outputs>
    pub implicit_outputs: RefCell<Vec<String>>,

    /// The list of dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub dependencies: RefCell<Vec<String>>,

    /// The list of implicit dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub implicit_dependencies: RefCell<Vec<String>>,

    /// The list of order-only dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub order_only_dependencies: RefCell<Vec<String>>,

    /// The list of validations.
    ///
    /// See <https://ninja-build.org/manual.html#validations>
    pub validations: RefCell<Vec<String>>,

    /// The list of variables, as an indented block
    pub variables: RefCell<Vec<Variable>>,
}

/// Trait for implementing build-specific variables
pub trait BuildVariables: Variables {
    /// Internal function for implementing variables for `build`
    fn as_build(&self) -> &Build;

    /// Specify dynamic dependency file
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).dyndep("bar");
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example
    ///   dyndep = bar
    /// "###);
    /// ```
    #[inline]
    fn dyndep<SDyndep>(&self, dyndep: SDyndep) -> &Self
where
        SDyndep: AsRef<str>,
    {
        self.variable("dyndep", dyndep)
    }

    /// Add explicit dependencies (inputs)
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// let mut rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz
    /// "###);
    fn with<SInputIter, SInput>(&self, inputs: SInputIter) -> &Self
where
        SInputIter: IntoIterator<Item = SInput>,
        SInput: AsRef<str>,
    {
        self.as_build().dependencies.borrow_mut().extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }
    /// Add implicit dependencies
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::Ninja;
    ///
    /// let mut ninja = Ninja::new();
    /// let mut rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"])
    ///     .with_implicit(["qux"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz | qux
    /// "###);
    fn with_implicit<SInputIter, SInput>(&self, inputs: SInputIter) -> &Self
where
        SInputIter: IntoIterator<Item = SInput>,
        SInput: AsRef<str>,
    {
        self.as_build().implicit_dependencies.borrow_mut().extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }
}


//             /// Add order-only dependencies
//             ///
//             /// See <https://ninja-build.org/manual.html#ref_dependencies>
//             ///
//             /// # Example
//             /// ```rust
//             /// use ninja_writer::Ninja;
//             ///
//             /// let mut ninja = Ninja::new();
//             /// let mut rule = ninja.rule("example", "...");
//             /// rule.build(["foo"]).with(["bar", "baz"])
//             ///     .with_order_only(["oo"])
//             ///     .with_implicit(["qux"]);
//             ///
//             /// assert_eq!(ninja.to_string(), r###"
//             /// rule example
//             ///   command = ...
//             ///
//             /// build foo: example bar baz | qux || oo
//             /// "###);
//             pub fn with_order_only<SInputIter, SInput>(mut self, inputs: SInputIter) -> Self
//             where
//                 SInputIter: IntoIterator<Item = SInput>,
//                 SInput: AsRef<str>,
//             {
//                 self.order_only_dependencies.extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
//                 self
//             }
//
//             /// Add validations
//             ///
//             /// See <https://ninja-build.org/manual.html#validations>
//             ///
//             /// # Example
//             /// ```rust
//             /// use ninja_writer::Ninja;
//             ///
//             /// let mut ninja = Ninja::new();
//             /// let mut rule = ninja.rule("example", "...");
//             /// rule.build(["foo"]).with(["bar", "baz"])
//             ///     .with_order_only(["oo"])
//             ///     .with_implicit(["qux"])
//             ///     .validations(["quux"]);
//             ///
//             /// assert_eq!(ninja.to_string(), r###"
//             /// rule example
//             ///   command = ...
//             ///
//             /// build foo: example bar baz | qux || oo |@ quux
//             /// "###);
//             pub fn validations<SValidationIter, SValidation>(mut self, validations: SValidationIter) -> Self
//             where
//                 SValidationIter: IntoIterator<Item = SValidation>,
//                 SValidation: AsRef<str>,
//             {
//                 self.validations.extend(validations.into_iter().map(|s| s.as_ref().to_owned()));
//                 self
//             }
//
//             /// Add validations
//             ///
//             /// See <https://ninja-build.org/manual.html#validations>
//             ///
//             /// # Example
//             /// ```rust
//             /// use ninja_writer::Ninja;
//             ///
//             /// let mut ninja = Ninja::new();
//             /// let mut rule = ninja.rule("example", "...");
//             /// rule.build(["foo"]).with(["bar", "baz"])
//             ///     .with_order_only(["oo"])
//             ///     .with_implicit(["qux"])
//             ///     .validations(["quux"])
//             ///     .output_implicit(["iii"]);
//             ///
//             /// assert_eq!(ninja.to_string(), r###"
//             /// rule example
//             ///   command = ...
//             ///
//             /// build foo | iii: example bar baz | qux || oo |@ quux
//             /// "###);
//             pub fn output_implicit<SOutputIter, SOutput>(mut self, outputs: SOutputIter) -> Self
//             where
//                 SOutputIter: IntoIterator<Item = SOutput>,
//                 SOutput: AsRef<str>,
//             {
//                 self.implicit_outputs.extend(outputs.into_iter().map(|s| s.as_ref().to_owned()));
//                 self
//             }
//         }
//     }
// }


/// Reference to a build statement
#[derive(Debug)]
pub struct BuildRef<'a, TList, TRc>
    where
        TList: StmtList<TRc=TRc>
        {pub inner:StmtRef<'a, TList, TRc>
    }
impl<'a, TList, TRc> Deref for BuildRef<'a, TList, TRc> where TList: StmtList<TRc=TRc>
, TRc: Deref<Target=Stmt> {
    type Target = Build;
    fn deref(&self) -> &Self::Target {
        match self.inner.deref().deref() {
            Stmt::Build(b) => b,
            _ => panic!("Expected build statement"),
        }
    }
}

// /// Wrapper for a [`Build`] that configure build edge with a reference
// pub struct BuildRef<'a>(pub(crate) &'a mut Build);
// impl AsRef<Build> for BuildRef<'_> {
//     #[inline]
//     fn as_ref(&self) -> &Build {
//         self.0
//     }
// }
// impl AsMut<Build> for BuildRef<'_> {
//     #[inline]
//     fn as_mut(&mut self) -> &mut Build {
//         self.0
//     }
// }
// impl Deref for BuildRef<'_> {
//     type Target = Build;
//     #[inline]
//     fn deref(&self) -> &Self::Target {
//         self.0
//     }
// }
// impl DerefMut for BuildRef<'_> {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.0
//     }
// }


// implement_rule_variables!(<'a> BuildRef<'a>);

impl Build {
    /// Create a new build with the given explicit outputs and rule
    pub fn new<SOutputIter, SOutput>(rule: &Rule, outputs: SOutputIter) -> Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        let outputs = outputs.into_iter().map(|s| s.as_ref().to_owned()).collect();
        Self {
            rule: Arc::clone(&rule.name),
            outputs: RefCell::new(outputs),
            implicit_outputs: Default::default(),
            dependencies: Default::default(),
            implicit_dependencies: Default::default(),
            order_only_dependencies: Default::default(),
            validations: Default::default(),
            variables: Default::default(),
        }
    }
}

impl Variables for Build {
    #[inline]
    fn add_variable_internal(&self, v: Variable) {
        self.variables.borrow_mut().push(v);
    }
}

impl BuildVariables for Build {
    fn as_build(&self) -> &Build {
        self
    }
}

impl RuleVariables for Build {}

impl Display for Build {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "build")?;
        for output in self.outputs.borrow().iter() {
            write!(f, " {}", output)?;
        }
        {
            let implicit_outputs = self.implicit_outputs.borrow();
            if !implicit_outputs.is_empty() {
                write!(f, " |")?;
                for output in implicit_outputs.iter() {
                    write!(f, " {}", output)?;
                }
            }
        }
        write!(f, ": {}", self.rule)?;
        for input in self.dependencies.borrow().iter() {
            write!(f, " {}", input)?;
        }
        {
            let implicit_dependencies = self.implicit_dependencies.borrow();
            if !implicit_dependencies.is_empty() {
                write!(f, " |")?;
                for input in implicit_dependencies.iter() {
                    write!(f, " {}", input)?;
                }
            }
        }
        {
            let order_only_dependencies = self.order_only_dependencies.borrow();
            if !order_only_dependencies.is_empty() {
                write!(f, " ||")?;
                for input in order_only_dependencies.iter() {
                    write!(f, " {}", input)?;
                }
            }
        }
        {
            let validations = self.validations.borrow();
            if !validations.is_empty() {
                write!(f, " |@")?;
                for input in validations.iter() {
                    write!(f, " {}", input)?;
                }
            }
        }
        writeln!(f)?;
        for variable in self.variables.borrow().iter() {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
