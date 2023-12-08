//! Implementation of the `build` keyword

use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter, Result};
use core::ops::{Deref, DerefMut};

use crate::util::{add_variable, implement_variables, Indented};
use crate::{implement_rule_variables, Build, Rule};

/// Wrapper for a [`Build`] that configure build edge with a reference
pub struct BuildRef<'a>(pub(crate) &'a mut Build);
impl AsRef<Build> for BuildRef<'_> {
    #[inline]
    fn as_ref(&self) -> &Build {
        self.0
    }
}
impl AsMut<Build> for BuildRef<'_> {
    #[inline]
    fn as_mut(&mut self) -> &mut Build {
        self.0
    }
}
impl Deref for BuildRef<'_> {
    type Target = Build;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}
impl DerefMut for BuildRef<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

macro_rules! implement_build_variables {
    ($($x:tt)*) => {
        /// Implementation for variables that are `build`-specific
        /// *(generated with `implement_build_variables` macro)*
        impl $($x)* {
            /// Specify dynamic dependency file
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// let mut rule = ninja.rule("example", "...");
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
            pub fn dyndep<SDyndep>(mut self, dyndep: SDyndep) -> Self
            where
                SDyndep: AsRef<str>,
            {
                add_variable!(self.variables, "dyndep", dyndep);
                self
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
            pub fn with<SInputIter, SInput>(mut self, inputs: SInputIter) -> Self
            where
                SInputIter: IntoIterator<Item = SInput>,
                SInput: AsRef<str>,
            {
                self.dependencies.extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
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
            pub fn with_implicit<SInputIter, SInput>(mut self, inputs: SInputIter) -> Self
            where
                SInputIter: IntoIterator<Item = SInput>,
                SInput: AsRef<str>,
            {
                self.implicit_dependencies.extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
                self
            }

            /// Add order-only dependencies
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
            ///     .with_order_only(["oo"])
            ///     .with_implicit(["qux"]);
            ///
            /// assert_eq!(ninja.to_string(), r###"
            /// rule example
            ///   command = ...
            ///
            /// build foo: example bar baz | qux || oo
            /// "###);
            pub fn with_order_only<SInputIter, SInput>(mut self, inputs: SInputIter) -> Self
            where
                SInputIter: IntoIterator<Item = SInput>,
                SInput: AsRef<str>,
            {
                self.order_only_dependencies.extend(inputs.into_iter().map(|s| s.as_ref().to_owned()));
                self
            }

            /// Add validations
            ///
            /// See <https://ninja-build.org/manual.html#validations>
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// let mut rule = ninja.rule("example", "...");
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
            pub fn validations<SValidationIter, SValidation>(mut self, validations: SValidationIter) -> Self
            where
                SValidationIter: IntoIterator<Item = SValidation>,
                SValidation: AsRef<str>,
            {
                self.validations.extend(validations.into_iter().map(|s| s.as_ref().to_owned()));
                self
            }

            /// Add validations
            ///
            /// See <https://ninja-build.org/manual.html#validations>
            ///
            /// # Example
            /// ```rust
            /// use ninja_writer::Ninja;
            ///
            /// let mut ninja = Ninja::new();
            /// let mut rule = ninja.rule("example", "...");
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
            pub fn output_implicit<SOutputIter, SOutput>(mut self, outputs: SOutputIter) -> Self
            where
                SOutputIter: IntoIterator<Item = SOutput>,
                SOutput: AsRef<str>,
            {
                self.implicit_outputs.extend(outputs.into_iter().map(|s| s.as_ref().to_owned()));
                self
            }
        }
    }
}

implement_build_variables!(<'a> BuildRef<'a>);
implement_variables!(<'a> BuildRef<'a>);
implement_rule_variables!(<'a> BuildRef<'a>);

impl Build {
    /// Create a new build with the given explicit outputs and rule
    pub fn new<SOutputIter, SOutput>(rule: &Rule, outputs: SOutputIter) -> Self
    where
        SOutputIter: IntoIterator<Item = SOutput>,
        SOutput: AsRef<str>,
    {
        Self {
            rule: Arc::clone(&rule.name),
            outputs: outputs.into_iter().map(|s| s.as_ref().to_owned()).collect(),
            implicit_outputs: Vec::new(),
            dependencies: Vec::new(),
            implicit_dependencies: Vec::new(),
            order_only_dependencies: Vec::new(),
            validations: Vec::new(),
            variables: Vec::new(),
        }
    }
}

implement_build_variables!(Build);
implement_variables!(Build);
implement_rule_variables!(Build);

impl Display for Build {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "build")?;
        for output in &self.outputs {
            write!(f, " {}", output)?;
        }
        if !self.implicit_outputs.is_empty() {
            write!(f, " |")?;
            for output in &self.implicit_outputs {
                write!(f, " {}", output)?;
            }
        }
        write!(f, ": {}", self.rule)?;
        for input in &self.dependencies {
            write!(f, " {}", input)?;
        }
        if !self.implicit_dependencies.is_empty() {
            write!(f, " |")?;
            for input in &self.implicit_dependencies {
                write!(f, " {}", input)?;
            }
        }
        if !self.order_only_dependencies.is_empty() {
            write!(f, " ||")?;
            for input in &self.order_only_dependencies {
                write!(f, " {}", input)?;
            }
        }
        if !self.validations.is_empty() {
            write!(f, " |@")?;
            for input in &self.validations {
                write!(f, " {}", input)?;
            }
        }
        writeln!(f)?;
        for variable in &self.variables {
            Indented(variable).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
