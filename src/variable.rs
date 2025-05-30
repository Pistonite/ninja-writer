//! Implementation of variables

use alloc::string::String;
use core::fmt::{Display, Formatter, Result};

use crate::ToArg;

/// A variable declaration (`name = value`)
///
/// See <https://ninja-build.org/manual.html#_variables>
///
/// # Escaping
/// Escaping must be done when constructing the variable. No escaping is done when serializing.
/// ```rust
/// use ninja_writer::Variable;
///
/// let var = Variable::new("foo", "I have a $ in me");
///
/// assert_eq!(var.to_string(), "foo = I have a $ in me");
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Variable {
    /// The name of the variable
    pub name: String,
    /// The value of the variable
    pub value: String,
}

impl Variable {
    /// Create a new variable declaration
    pub fn new(name: impl ToArg, value: impl ToArg) -> Self {
        Self {
            name: name.to_arg(),
            value: value.to_arg(),
        }
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{} = {}", self.name, self.value)
    }
}

/// Convienience trait to implement types that supports variables
pub trait Variables: Sized {
    /// Add a variable
    ///
    /// This is an internal method to add a variable to the current scope.
    fn add_variable_internal(&self, v: Variable);

    /// Add a variable to the current scope
    fn variable(self, name: impl ToArg, value: impl ToArg) -> Self {
        self.add_variable_internal(Variable::new(name, value));
        self
    }
}
