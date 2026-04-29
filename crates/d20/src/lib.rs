//!
//! A Rust port of the [d20](https://d20.readthedocs.io/en/latest/start.html) dice
//! engine.
//!

#![feature(never_type)]

pub mod eval;
pub mod parser;
pub mod provider;
pub mod utils;

use std::rc::Rc;

pub use parser::parse;
pub use provider::DiceRoller;

#[cfg(feature = "rand")]
pub use provider::local_rng::LocalRng;

pub mod reexport {
    #[cfg(feature = "rand")]
    pub use rand;
}

/// A dice expression.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DExpr {
    Literal(Literal),
    Dice(Dice),
    UnaryOperation(UnaryOperator, Box<Self>),
    Set(Vec<Self>),
    SetOperation(Box<Self>, SetOp),
    BinaryOperation(Box<Self>, BinaryOperator, Box<Self>),
    Labeled(Box<Self>, Label),
}

/// A semantic label for a sub-expression.
///
/// Labels are ignored for [std::cmp::PartialEq] (`==`).
#[derive(Debug, Clone)]
pub struct Label(pub Rc<dyn xander_runtime::ui::Ui>);

/// A set of die.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dice {
    pub qty: Option<Int>,
    pub sides: Int,
}

/// A literal number.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Literal {
    Int(Int),
    Decimal(Decimal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Int(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Decimal(pub f64);

/// These operations can be performed on dice and sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetOp(pub SetOperator, pub Selection);

/// [SetOperator]s are always followed by a [Selector], and operate on the items in the set that match the selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetOperator {
    /// Keeps all matched values.
    Keep,
    /// Drops all matched values.
    Drop,
    /// Rerolls all matched values until none match. (Dice only)
    Reroll,
    /// Rerolls all matched values once. (Dice only)
    RerollOnce,
    /// Rerolls up to one matched value once, keeping the original roll. (Dice only)
    RerollAndAdd,
    /// Rolls another die for each matched value. (Dice only)
    ExplodeOn,
    /// Sets the minimum value of each die. (Dice only)
    Minimum,
    /// Sets the maximum value of each die. (Dice only)
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selection(pub Selector, pub Int);

/// [Selector]s select from the remaining kept values in a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Selector {
    /// All values in this set that are literally this value.
    Literal,
    /// The highest X values in the set.
    Highest,
    /// The lowest X values in the set.
    Lowest,
    /// All values in this set greater than X.
    GreaterThan,
    /// All values in this set less than X.
    LessThan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnaryOperator {
    /// Does nothing.
    Positive,
    /// The negative value of X.
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BinaryOperator {
    /// Multiplication
    Mul,
    /// Division
    Div,
    /// Int Division
    IntDiv,
    /// Modulo
    Rem,
    /// Addition
    Add,
    /// Subtraction
    Sub,
    /// Equality
    Eq,
    /// Greater/Equal
    GtE,
    /// Less/Equal
    LtE,
    /// Greater Than
    Gt,
    /// Less Than
    Lt,
    /// Inequality
    NEq,
}

impl BinaryOperator {
    pub fn precedence(&self) -> usize {
        use BinaryOperator::*;

        match self {
            /* Product-level */
            Mul | Div | IntDiv | Rem => 0,
            /* Sum-level */
            Add | Sub => 1,
            /* Logic-level */
            Eq | NEq | GtE | LtE | Gt | Lt => 2,
        }
    }
}

impl Dice {
    pub fn qty(&self) -> u32 {
        //
        self.qty.map_or(1, |Int(sides)| sides)
    }
}

pub const D4: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(4),
});
pub const D6: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(6),
});
pub const D8: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(8),
});
pub const D10: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(10),
});
pub const D12: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(12),
});
pub const D20: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(20),
});
pub const D100: DExpr = DExpr::Dice(Dice {
    qty: None,
    sides: Int(100),
});

pub use crate::eval::*;
