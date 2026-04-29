// TODO: Consider maybe making a new-type instead (TestD20)

use std::rc::{Rc, Weak};

use xander_runtime::ui;

#[derive(Debug, PartialEq)]
pub(in crate::engine::game::stats::d20_test) struct TestRoll;
impl ui::Ui for TestRoll {}

/// Grant advantage to test rolls.
///
/// The [d20::DExpr] must have originated from a [super::D20Test].
#[derive(Debug)]
pub struct Advantage {
    pub reason: Option<Weak<dyn ui::Ui>>,
}

impl ui::Ui for Advantage {}

#[derive(Debug)]
pub struct Disadvantage {
    pub reason: Option<Weak<dyn ui::Ui>>,
}

impl ui::Ui for Disadvantage {}

#[derive(Debug, PartialEq)]
struct CancelledOut;
impl ui::Ui for CancelledOut {}

/// Returns *new* [d20::DExpr] rolls with [Advantage] or [Disadvantage] imposed.
pub trait D20TestRoll: Sized {
    #[must_use]
    fn grant(&self, advantage: Advantage) -> Self;

    #[must_use]
    fn impose(&self, disadvantage: Disadvantage) -> Self;
}

impl D20TestRoll for d20::DExpr {
    fn grant(&self, advantage: Advantage) -> Self {
        <Advantage as RollEffect>::apply(advantage, self.clone())
    }

    fn impose(&self, disadvantage: Disadvantage) -> Self {
        <Disadvantage as RollEffect>::apply(disadvantage, self.clone())
    }
}

trait RollEffect: ui::Ui + Sized {
    type Opposite: RollEffect;
    const OPERATION: d20::SetOp;

    fn apply(self, mut roll: d20::DExpr) -> d20::DExpr {
        // "[Cancelling out] is even true if multiple circumstances impose
        // disadvantage and only one grants advantage and vice versa."
        if find_labelled_mut::<CancelledOut>(&mut roll).is_some() {
            return roll;
        }

        // "If multiple situations affect a roll and all grant advantage
        // [or all impose disadvantage] on it, you still only roll two d20s."
        if find_labelled_mut::<Self>(&mut roll).is_some() {
            return roll;
        }

        // "If circumstances cause a roll to both have advantage and disadvantage,
        // the roll has neither of them."
        if let Some(affected_roll) = find_labelled_mut::<Self::Opposite>(&mut roll) {
            // Just replace the whole thing.
            *affected_roll = d20::D20
                .label(Rc::new(TestRoll))
                .label(Rc::new(CancelledOut));

            return roll;
        }

        // Enact the roll effect.
        if find_labelled_mut::<TestRoll>(&mut roll).is_some() {
            let test_roll = d20::DExpr::Dice(d20::Dice {
                qty: Some(d20::Int(2)),
                sides: d20::Int(20),
            })
            .label(Rc::new(TestRoll));

            return d20::DExpr::SetOperation(Box::new(test_roll), Self::OPERATION)
                .label(Rc::new(self));
        }

        // TODO: handle the annoying cases under "Interactions with Rerolls"
        unimplemented!("You have tried to grant/impose advantage/disadvantage on a non-test roll")
    }
}

impl RollEffect for Advantage {
    type Opposite = Disadvantage;
    const OPERATION: d20::SetOp = d20::SetOp(
        d20::SetOperator::Keep,
        d20::Selection(d20::Selector::Highest, d20::Int(1)),
    );
}

impl RollEffect for Disadvantage {
    type Opposite = Advantage;
    const OPERATION: d20::SetOp = d20::SetOp(
        d20::SetOperator::Keep,
        d20::Selection(d20::Selector::Lowest, d20::Int(1)),
    );
}

macro_rules! find_labelled_impl {
    ($(@$mut: ident,)? $ty: ident, $fn_name: ident, $iter_fn: ident) => {
        #[allow(unused)]
        pub fn $fn_name<L>(expr: &$($mut)? d20::$ty) -> Option<&$($mut)? d20::$ty>
        where
            L: ui::Ui,
        {
            let labelled_expr = match expr {
                d20::$ty::Literal(..) => return None,
                d20::$ty::Dice(..) => return None,
                d20::$ty::UnaryOperation(_, expr) => return $fn_name::<L>(expr),
                d20::$ty::Set(exprs) => {
                    return exprs
                        .$iter_fn()
                        .filter_map(|expr| $fn_name::<L>(expr))
                        .next();
                }
                d20::$ty::SetOperation(expr, _) => return $fn_name::<L>(expr),
                d20::$ty::BinaryOperation(lhs, _, rhs) => {
                    return $fn_name::<L>(lhs).or_else(|| $fn_name::<L>(rhs));
                }
                expr @ d20::$ty::Labeled(..) => expr,
            };

            // If the label doesn't match...
            if let d20::$ty::Labeled(_, with_label) = labelled_expr
                && !with_label.0.is::<L>()
            {
                return None;
            }

            Some(labelled_expr)
        }
    };
}
find_labelled_impl!(@mut, DExpr, find_labelled_mut, iter_mut);
find_labelled_impl!(DExpr, find_labelled, iter);

find_labelled_impl!(ValTree, find_labelled_val, iter);
find_labelled_impl!(@mut, ValTree, find_labelled_val_mut, iter_mut);

#[cfg(test)]
mod tests {
    use super::{Advantage, D20TestRoll};
    use crate::engine::game::stats::d20_test::d20_test;

    use super::Disadvantage;

    #[test]
    fn roll_effect_properties() {
        let d20 = d20_test();

        let adv = d20.clone().grant(Advantage { reason: None });
        let adv_adv = adv.clone().grant(Advantage { reason: None });
        assert_eq!(adv, adv_adv); // Idempotence

        let dis = d20.clone().impose(Disadvantage { reason: None });
        let dis_dis = dis.clone().impose(Disadvantage { reason: None });
        assert_eq!(dis, dis_dis); // Idempotence

        let adv_dis = adv.clone().impose(Disadvantage { reason: None });
        let dis_adv = dis.clone().grant(Advantage { reason: None });
        assert_eq!(adv_dis, dis_adv); // Symmetry when cancelling out.
    }
}
