//! Implementation of the `build` keyword

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt::{Display, Formatter, Result};
use core::ops::Deref;

use crate::stmt::{Stmt, StmtRef};
use crate::util::{AddOnlyVec, Indented, RefCounted};
use crate::{MaybeOs, MaybeOsDisplay};
use crate::{Rule, RuleVariables, Variable, Variables};

/// A build edge, as defined by the `build` keyword
///
/// See <https://ninja-build.org/manual.html#_build_statements>
///
/// # Example
/// Since build edges are tied to rules, use [`RuleRef::build`](crate::RuleRef::build) to create them.
/// ```rust
/// use ninja_writer::*;
///
/// let ninja = Ninja::new();
/// let cc = ninja.rule("cc", "gcc $cflags -c $in -o $out");
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
#[derive(Debug)]
pub struct Build {
    /// The rule name
    pub rule: RefCounted<String>,

    /// The list of outputs, as defined by `build <outputs>:`
    pub outputs: AddOnlyVec<MaybeOs!(String)>,

    /// The list of implicit outputs.
    ///
    /// See <https://ninja-build.org/manual.html#ref_outputs>
    pub implicit_outputs: AddOnlyVec<MaybeOs!(String)>,

    /// The list of dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub dependencies: AddOnlyVec<MaybeOs!(String)>,

    /// The list of implicit dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub implicit_dependencies: AddOnlyVec<MaybeOs!(String)>,

    /// The list of order-only dependencies (inputs).
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    pub order_only_dependencies: AddOnlyVec<MaybeOs!(String)>,

    /// The list of validations.
    ///
    /// See <https://ninja-build.org/manual.html#validations>
    pub validations: AddOnlyVec<MaybeOs!(String)>,

    /// The list of variables, as an indented block
    pub variables: AddOnlyVec<Variable>,
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
    fn dyndep<SDyndep>(self, dyndep: SDyndep) -> Self
    where
        SDyndep: AsRef<MaybeOs!(str)>,
    {
        self.variable("dyndep", dyndep)
    }

    /// Add explicit dependencies (inputs)
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz
    /// "###);
    fn with<SInputIter, SInput>(self, inputs: SInputIter) -> Self
    where
        SInputIter: IntoIterator<Item = SInput>,
        SInput: AsRef<MaybeOs!(str)>,
    {
        self.as_build()
            .dependencies
            .extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Add implicit dependencies
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"])
    ///     .with_implicit(["qux"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz | qux
    /// "###);
    fn with_implicit<SInputIter, SInput>(self, inputs: SInputIter) -> Self
    where
        SInputIter: IntoIterator<Item = SInput>,
        SInput: AsRef<MaybeOs!(str)>,
    {
        self.as_build()
            .implicit_dependencies
            .extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Add order-only dependencies
    ///
    /// See <https://ninja-build.org/manual.html#ref_dependencies>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"])
    ///     .with_order_only(["oo"])
    ///     .with_implicit(["qux"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz | qux || oo
    /// "###);
    fn with_order_only<SInputIter, SInput>(self, inputs: SInputIter) -> Self
    where
        SInputIter: IntoIterator<Item = SInput>,
        SInput: AsRef<MaybeOs!(str)>,
    {
        self.as_build()
            .order_only_dependencies
            .extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Add validations
    ///
    /// See <https://ninja-build.org/manual.html#validations>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"])
    ///     .with_order_only(["oo"])
    ///     .with_implicit(["qux"])
    ///     .validations(["quux"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo: example bar baz | qux || oo |@ quux
    /// "###);
    fn validations<SValidationIter, SValidation>(self, validations: SValidationIter) -> Self
    where
        SValidationIter: IntoIterator<Item = SValidation>,
        SValidation: AsRef<MaybeOs!(str)>,
    {
        self.as_build()
            .validations
            .extend(validations.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Add validations
    ///
    /// See <https://ninja-build.org/manual.html#validations>
    ///
    /// # Example
    /// ```rust
    /// use ninja_writer::*;
    ///
    /// let ninja = Ninja::new();
    /// let rule = ninja.rule("example", "...");
    /// rule.build(["foo"]).with(["bar", "baz"])
    ///     .with_order_only(["oo"])
    ///     .with_implicit(["qux"])
    ///     .validations(["quux"])
    ///     .output_implicit(["iii"]);
    ///
    /// assert_eq!(ninja.to_string(), r###"
    /// rule example
    ///   command = ...
    ///
    /// build foo | iii: example bar baz | qux || oo |@ quux
    /// "###);
    fn output_implicit<SOutputIter, SOutput>(self, outputs: SOutputIter) -> Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<MaybeOs!(str)>,
    {
        self.as_build()
            .implicit_outputs
            .extend(outputs.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }
}

/// Reference to a build statement
#[derive(Debug, Clone)]
pub struct BuildRef(pub(crate) StmtRef);

impl Deref for BuildRef {
    type Target = Build;
    fn deref(&self) -> &Self::Target {
        match self.0.deref().deref() {
            Stmt::Build(b) => b,
            _ => panic!("Expected build statement"),
        }
    }
}

impl AsRef<Build> for BuildRef {
    #[inline]
    fn as_ref(&self) -> &Build {
        self.deref()
    }
}

impl Build {
    /// Create a new build with the given explicit outputs and rule
    pub fn new<SOutputIter, SOutput>(rule: &Rule, outputs: SOutputIter) -> Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<MaybeOs!(str)>,
    {
        let self_outputs = AddOnlyVec::new();
        self_outputs.extend(outputs.into_iter().map(|s| s.as_ref().to_owned()));
        Self {
            rule: RefCounted::clone(&rule.name),
            outputs: self_outputs,
            implicit_outputs: AddOnlyVec::new(),
            dependencies: AddOnlyVec::new(),
            implicit_dependencies: AddOnlyVec::new(),
            order_only_dependencies: AddOnlyVec::new(),
            validations: AddOnlyVec::new(),
            variables: AddOnlyVec::new(),
        }
    }
}

impl Variables for Build {
    #[inline]
    fn add_variable_internal(&self, v: Variable) {
        self.variables.add(v);
    }
}

impl BuildVariables for Build {
    fn as_build(&self) -> &Build {
        self
    }
}

impl RuleVariables for Build {}

impl Variables for BuildRef {
    #[inline]
    fn add_variable_internal(&self, v: Variable) {
        self.deref().variables.add(v);
    }
}

impl BuildVariables for BuildRef {
    fn as_build(&self) -> &Build {
        self.deref()
    }
}

impl RuleVariables for BuildRef {}

impl Display for Build {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "build")?;
        for output in self.outputs.inner().iter() {
            write!(f, " {}", MaybeOsDisplay!(output))?;
        }
        {
            let implicit_outputs = self.implicit_outputs.inner();
            if !implicit_outputs.is_empty() {
                write!(f, " |")?;
                for output in implicit_outputs.iter() {
                    write!(f, " {}", MaybeOsDisplay!(output))?;
                }
            }
        }
        write!(f, ": {}", self.rule)?;
        for input in self.dependencies.inner().iter() {
            write!(f, " {}", MaybeOsDisplay!(input))?;
        }
        {
            let implicit_dependencies = self.implicit_dependencies.inner();
            if !implicit_dependencies.is_empty() {
                write!(f, " |")?;
                for input in implicit_dependencies.iter() {
                    write!(f, " {}", MaybeOsDisplay!(input))?;
                }
            }
        }
        {
            let order_only_dependencies = self.order_only_dependencies.inner();
            if !order_only_dependencies.is_empty() {
                write!(f, " ||")?;
                for input in order_only_dependencies.iter() {
                    write!(f, " {}", MaybeOsDisplay!(input))?;
                }
            }
        }
        {
            let validations = self.validations.inner();
            if !validations.is_empty() {
                write!(f, " |@")?;
                for input in validations.iter() {
                    write!(f, " {}", MaybeOsDisplay!(input))?;
                }
            }
        }
        writeln!(f)?;
        for variable in self.variables.inner().iter() {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
