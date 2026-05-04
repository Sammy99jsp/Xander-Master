use crate::{
    BinaryOperator as Op, DExpr, Dice, Int, Label, Labeled, Literal, UnaryOperator, ValTree,
};

// Labels / UI

impl Label {
    pub const fn new<UI: xander_runtime::ui::Ui>(ui: Rc<UI>) -> Self {
        Self(Some(ui))
    }

    pub const fn empty() -> Self {
        Self(None)
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
        Self::Labeled(Labeled(Box::new(self), Label::new(ui)))
    }
}

impl ValTree {
    pub fn label<UI>(self, ui: Rc<UI>) -> Self
    where
        UI: xander_runtime::ui::Ui,
    {
        Self::Labeled(Labeled(Box::new(self), Label::new(ui)))
    }
}

impl xander_runtime::ui::Ui for DExpr {}
impl xander_runtime::ui::Ui for ValTree {}

// DEFAULTS

impl DExpr {
    pub const ZERO: Self = DExpr::Literal(Literal::Int(Int(0)));
}

impl Default for DExpr {
    fn default() -> Self {
        Self::ZERO
    }
}

impl ValTree {
    pub const ZERO: Self = ValTree::Literal(Literal::Int(Int(0)));
}

impl Default for ValTree {
    fn default() -> Self {
        Self::ZERO
    }
}

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
    ops::{Add, AddAssign, Div, Mul, Neg, Rem, Sub, SubAssign},
    rc::Rc,
};

macro_rules! op {
    (Neg, neg) => {
        impl Neg for DExpr {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self::UnaryOperation(UnaryOperator::Negative, Box::new(self))
            }
        }

        impl Neg for ValTree {
            type Output = Self;

            fn neg(self) -> Self::Output {
                Self::UnaryOperation(UnaryOperator::Negative, Box::new(self))
            }
        }
    };
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

op!(Neg, neg);
op!(Add, add);
op!(Sub, sub);
op!(Mul, mul);
op!(Div, div);
op!(Rem, rem);

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

// Finding things with labels

macro_rules! find_labelled_impl {
    ($(@$mut: ident,)? $ty: ident, $fn_name: ident, $iter_fn: ident) => {

        impl $ty {
            pub fn $fn_name<L>(&$($mut)? self) -> Option<&$($mut)? Self>
            where
                L: ::xander_runtime::ui::Ui,
            {
                let labelled_expr = match self {
                    $ty::Literal(..) => return None,
                    $ty::Dice(..) => return None,
                    $ty::UnaryOperation(_, expr) => return Self::$fn_name::<L>(expr),
                    $ty::Set(exprs) => {
                        return exprs
                            .$iter_fn()
                            .filter_map(|expr| Self::$fn_name::<L>(expr))
                            .next();
                    }
                    $ty::SetOperation(expr, _) => return Self::$fn_name::<L>(expr),
                    $ty::BinaryOperation(lhs, _, rhs) => {
                        return Self::$fn_name::<L>(lhs).or_else(|| Self::$fn_name::<L>(rhs));
                    }
                    expr @ $ty::Labeled(..) => expr,
                };

                // If the label doesn't match...
                if let $ty::Labeled(Labeled(_, with_label)) = labelled_expr
                    && !with_label.0.as_ref().is_some_and(|l| l.is::<L>())
                {
                    return None;
                }

                Some(labelled_expr)
            }

        }
    };
}
find_labelled_impl!(@mut, DExpr, find_labelled_mut, iter_mut);
find_labelled_impl!(DExpr, find_labelled, iter);

find_labelled_impl!(ValTree, find_labelled_val, iter);
find_labelled_impl!(@mut, ValTree, find_labelled_val_mut, iter_mut);

macro_rules! traverse_impl {
    ($(@$mut: ident,)? $ty: ident, $fn_name: ident, $iter_fn: ident) => {
        impl $ty {
            pub fn $fn_name<F>(&$($mut)? self, mut f: F)
            where
                F: for<'a> FnMut(&'a $($mut)? $ty),
            {
                fn _inner<F>(expr: &$($mut)? $ty, f: & mut F)
                where
                    F: for<'a> FnMut(&'a $($mut)? $ty),
                {
                    match expr {
                        expr @ ($ty::Literal(_) | $ty::Dice(_))
                        | $ty::Labeled(Labeled(box expr, _))
                        | $ty::UnaryOperation(_, box expr)
                        | $ty::SetOperation(box expr, _) => f(expr),

                        $ty::BinaryOperation(lhs, _, rhs) => {
                            _inner(lhs, f);
                            _inner(rhs, f)
                        }
                        $ty::Set(exprs) => exprs.$iter_fn().for_each(|expr| _inner(expr, f)),
                    }
                }

                _inner(self, &mut f);
            }
        }
    };
}

traverse_impl!(@mut, DExpr, traverse_mut, iter_mut);
traverse_impl!(DExpr, traverse, iter);

traverse_impl!(@mut, ValTree, traverse_mut, iter_mut);
traverse_impl!(ValTree, traverse, iter);

// Other utilities

macro_rules! etc {
    ($ty: ident) => {
        impl $ty {
            pub fn modify_in_place(&mut self, func: impl FnOnce(Self) -> Self) {
                replace_expr(self, func);
            }

            pub fn int_div(self, div: i32) -> Self {
                Self::BinaryOperation(Box::new(self), Op::IntDiv, Box::new(div.into()))
            }
        }
    };
}

etc!(ValTree);
etc!(DExpr);
