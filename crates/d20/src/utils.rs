use crate::{BinaryOperator as Op, DExpr, Dice, Int, Label, Literal, UnaryOperator, ValTree};

// Labels / UI

impl Label {
    pub const fn new<UI: xander_runtime::ui::Ui>(ui: Rc<UI>) -> Self {
        Self(ui)
    }
}

impl PartialEq for Label {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl PartialOrd for Label {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl DExpr {
    pub fn label<UI>(self, ui: Rc<UI>) -> Self
    where
        UI: xander_runtime::ui::Ui,
    {
        Self::Labeled(Box::new(self), Label::new(ui))
    }
}

impl ValTree {
    pub fn label<UI>(self, ui: Rc<UI>) -> Self
    where
        UI: xander_runtime::ui::Ui,
    {
        Self::Labeled(Box::new(self), Label::new(ui))
    }
}

impl xander_runtime::ui::Ui for DExpr {}
impl xander_runtime::ui::Ui for ValTree {}

// Conversions

impl From<Dice> for DExpr {
    fn from(dice: Dice) -> Self {
        DExpr::Dice(dice)
    }
}

impl From<i32> for DExpr {
    fn from(value: i32) -> Self {
        let expr = DExpr::Literal(Literal::Int(Int(value.unsigned_abs())));

        if value.is_negative() {
            DExpr::UnaryOperation(UnaryOperator::Negative, Box::new(expr))
        } else {
            expr
        }
    }
}

impl From<i32> for ValTree {
    fn from(value: i32) -> Self {
        let expr = ValTree::Literal(Literal::Int(Int(value.unsigned_abs())));

        if value.is_negative() {
            ValTree::UnaryOperation(UnaryOperator::Negative, Box::new(expr))
        } else {
            expr
        }
    }
}

// Arithmetic

use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign},
    rc::Rc,
};

macro_rules! op {
    ($op: ident, $fn_name: ident) => {
        impl $op<DExpr> for DExpr {
            type Output = DExpr;

            fn $fn_name(self, rhs: DExpr) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self), Op::$op, Box::new(rhs))
            }
        }

        impl $op<Dice> for DExpr {
            type Output = DExpr;

            fn $fn_name(self, rhs: Dice) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self), Op::$op, Box::new(rhs.into()))
            }
        }

        impl $op<i32> for DExpr {
            type Output = DExpr;

            fn $fn_name(self, rhs: i32) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self), Op::$op, Box::new(rhs.into()))
            }
        }

        impl $op<DExpr> for i32 {
            type Output = DExpr;

            fn $fn_name(self, rhs: DExpr) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs))
            }
        }

        impl $op<Dice> for i32 {
            type Output = DExpr;

            fn $fn_name(self, rhs: Dice) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs.into()))
            }
        }

        impl $op<DExpr> for Dice {
            type Output = DExpr;

            fn $fn_name(self, rhs: DExpr) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs))
            }
        }

        impl $op<Dice> for Dice {
            type Output = DExpr;

            fn $fn_name(self, rhs: Dice) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs.into()))
            }
        }

        impl $op<i32> for Dice {
            type Output = DExpr;

            fn $fn_name(self, rhs: i32) -> Self::Output {
                DExpr::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs.into()))
            }
        }

        // Val Tree

        impl $op<ValTree> for ValTree {
            type Output = ValTree;

            fn $fn_name(self, rhs: ValTree) -> Self::Output {
                ValTree::BinaryOperation(Box::new(self), Op::$op, Box::new(rhs))
            }
        }
        impl $op<i32> for ValTree {
            type Output = ValTree;

            fn $fn_name(self, rhs: i32) -> Self::Output {
                ValTree::BinaryOperation(Box::new(self), Op::$op, Box::new(rhs.into()))
            }
        }

        impl $op<ValTree> for i32 {
            type Output = ValTree;

            fn $fn_name(self, rhs: ValTree) -> Self::Output {
                ValTree::BinaryOperation(Box::new(self.into()), Op::$op, Box::new(rhs))
            }
        }
    };
}

op!(Add, add);
op!(Sub, sub);
op!(Mul, mul);
op!(Div, div);
op!(Rem, rem);

impl ValTree {
    pub fn modify_in_place(&mut self, func: impl FnOnce(Self) -> Self) {
        replace_expr(self, func);
    }

    pub fn int_div(self, div: i32) -> Self {
        Self::BinaryOperation(Box::new(self), Op::IntDiv, Box::new(div.into()))
    }
}

fn replace_expr<A: Clone>(expr: &mut A, func: impl FnOnce(A) -> A) {
    *expr = func(expr.clone());
}

impl AddAssign for DExpr {
    fn add_assign(&mut self, rhs: Self) {
        replace_expr(self, move |expr| expr + rhs);
    }
}

impl AddAssign for ValTree {
    fn add_assign(&mut self, rhs: Self) {
        replace_expr(self, move |val| val + rhs);
    }
}

impl SubAssign for DExpr {
    fn sub_assign(&mut self, rhs: Self) {
        replace_expr(self, move |expr| expr - rhs);
    }
}

impl SubAssign for ValTree {
    fn sub_assign(&mut self, rhs: Self) {
        replace_expr(self, move |expr| expr - rhs);
    }
}
