use std::cmp::Ordering;

use crate::{DExpr, Decimal, Int, Label, Selector, SetOp};

use super::{BinaryOperator, Literal, Selection, SetOperator, UnaryOperator};

/// An evaluated [DExpr].
///
/// Use [ValTree::result] to get the result as an [i32].
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum UnevalTree {
    Literal(Literal),
    Roll(DiceRoll),
    UnaryOperation(UnaryOperator, Box<Self>),
    Set(Vec<Self>),
    ValSetOperation(Box<Self>, ValSetOp),
    BinaryOperation(Box<Self>, BinaryOperator, Box<Self>),
    Labeled(Box<Self>, Label),
    RollSetOperation(DiceRoll, RollSetOp),
}

/// An evaluated [DExpr].
///
/// Use [ValTree::result] to get the result as an [i32].
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ValTree {
    Literal(Literal),
    Dice(DiceRoll),
    UnaryOperation(UnaryOperator, Box<Self>),
    Set(Vec<Self>),
    SetOperation(Box<Self>, ValSetOp),
    BinaryOperation(Box<Self>, BinaryOperator, Box<Self>),
    Labeled(Box<Self>, Label),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DieRoll {
    Normal(u32),
    Replace {
        original: u32,
        op: RollSetOp,
        new: Box<Self>,
    },
    InAddition {
        original: u32,
        op: RollSetOp,
        new: Box<Self>,
    },
}

impl DieRoll {
    pub fn value(&self) -> i32 {
        match self {
            DieRoll::Normal(val) => *val as i32,
            // NOTE: Add `become` keyword when it becomes stable.
            DieRoll::Replace { new, .. } => new.value(),
            DieRoll::InAddition { original, new, .. } => *original as i32 + new.value(),
        }
    }

    pub fn last_roll(&self) -> u32 {
        match self {
            DieRoll::Normal(roll) => *roll,
            DieRoll::Replace { new, .. } => new.last_roll(),
            DieRoll::InAddition { new, .. } => new.last_roll(),
        }
    }

    #[inline]
    fn leaf_mut(&mut self, func: impl FnOnce(&mut Self, u32)) -> &mut Self {
        match self {
            old @ DieRoll::Normal(_) => {
                let original = match old {
                    DieRoll::Normal(original) => *original,
                    _ => unreachable!(),
                };

                func(old, original);

                old
            }
            DieRoll::Replace { new, .. } => new.leaf_mut(func),
            DieRoll::InAddition { new, .. } => new.leaf_mut(func),
        }
    }

    pub fn in_addtion_with(&mut self, op: RollSetOp, new: u32) -> &mut Self {
        let new = self.leaf_mut(move |old, original| {
            *old = Self::InAddition {
                original,
                op,
                new: Box::new(Self::Normal(new)),
            }
        });

        match new {
            Self::InAddition { new, .. } => new.as_mut(),
            _ => unreachable!(),
        }
    }

    pub fn replace_with(&mut self, op: RollSetOp, new: u32) -> &mut Self {
        let new = self.leaf_mut(move |old, original| {
            *old = Self::Replace {
                original,
                op,
                new: Box::new(Self::Normal(new)),
            }
        });

        match new {
            Self::Replace { new, .. } => new.as_mut(),
            _ => unreachable!(),
        }
    }

    pub fn update_with(&mut self, op: RollSetOp, new_roll: u32) -> &mut Self {
        use RollSetOperator::*;
        match op {
            op @ RollSetOp(Reroll, _) => self.replace_with(op, new_roll),
            op @ RollSetOp(RerollOnce, _) => self.replace_with(op, new_roll),
            op @ RollSetOp(RerollAndAdd, _) => self.in_addtion_with(op, new_roll),
            op @ RollSetOp(ExplodeOn, _) => self.in_addtion_with(op, new_roll),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiceRoll {
    pub dice: crate::Dice,
    pub results: Vec<DieRoll>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValSetOp(pub ValSetOperator, pub Selection);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RollSetOp(pub RollSetOperator, pub Selection);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValSetOperator {
    /// Keeps all matched values.
    Keep,
    /// Drops all matched values.
    Drop,
    /// Sets the minimum value of each die. (Dice only)
    Minimum,
    /// Sets the maximum value of each die. (Dice only)
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RollSetOperator {
    /// Rerolls all matched values until none match. (Dice only)
    Reroll,
    /// Rerolls all matched values once. (Dice only)
    RerollOnce,
    /// Rerolls up to one matched value once, keeping the original roll. (Dice only)
    RerollAndAdd,
    /// Rolls another die for each matched value. (Dice only)
    ExplodeOn,
}

impl TryFrom<SetOperator> for ValSetOperator {
    type Error = RollSetOperator;

    fn try_from(value: SetOperator) -> Result<Self, Self::Error> {
        match value {
            SetOperator::Keep => Ok(Self::Keep),
            SetOperator::Drop => Ok(Self::Drop),
            SetOperator::Minimum => Ok(Self::Minimum),
            SetOperator::Maximum => Ok(Self::Maximum),
            SetOperator::Reroll => Err(RollSetOperator::Reroll),
            SetOperator::RerollOnce => Err(RollSetOperator::RerollOnce),
            SetOperator::RerollAndAdd => Err(RollSetOperator::RerollAndAdd),
            SetOperator::ExplodeOn => Err(RollSetOperator::ExplodeOn),
        }
    }
}

impl DExpr {
    #[rustfmt::skip]
    pub(crate) fn to_uneval(&self) -> Result<UnevalTree, ValTreeError> {
        Ok(match self {
            DExpr::Literal(literal) => UnevalTree::Literal(literal.clone()),
            DExpr::Dice(dice) => UnevalTree::Roll(DiceRoll { dice: *dice, results: Vec::new() }),
            DExpr::UnaryOperation(unary_operator, expr) => UnevalTree::UnaryOperation(*unary_operator, Box::new(expr.to_uneval()?)),
            DExpr::Labeled(expr, lbl) => UnevalTree::Labeled(Box::new(expr.to_uneval()?), lbl.clone()),
            DExpr::Set(exprs) => UnevalTree::Set(
                exprs
                    .iter()
                    .map(|expr| expr.to_uneval())
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            DExpr::BinaryOperation(lhs, binary_operator, rhs) => UnevalTree::BinaryOperation(
                Box::new(lhs.to_uneval()?),
                *binary_operator,
                Box::new(rhs.to_uneval()?),
            ),
            DExpr::SetOperation(expr, op @ SetOp(operator, sel)) => {
                use SetOperator::*;
                match (operator, expr.as_ref()) {
                    (Keep,    expr)                  => UnevalTree::ValSetOperation(Box::new(expr.to_uneval()?), ValSetOp(ValSetOperator::Keep, *sel)),
                    (Drop,    expr)                  => UnevalTree::ValSetOperation(Box::new(expr.to_uneval()?), ValSetOp(ValSetOperator::Drop, *sel)),
                    (Minimum, expr @ DExpr::Dice(_)) => UnevalTree::ValSetOperation(Box::new(expr.to_uneval()?), ValSetOp(ValSetOperator::Minimum, *sel)),
                    (Maximum, expr @ DExpr::Dice(_)) => UnevalTree::ValSetOperation(Box::new(expr.to_uneval()?), ValSetOp(ValSetOperator::Maximum, *sel)),

                    /* RollSetOp */
                    (Reroll,       DExpr::Dice(dice)) => UnevalTree::RollSetOperation(DiceRoll { dice: *dice, results: Vec::new() }, RollSetOp(RollSetOperator::Reroll, *sel)),
                    (RerollOnce,   DExpr::Dice(dice)) => UnevalTree::RollSetOperation(DiceRoll { dice: *dice, results: Vec::new() }, RollSetOp(RollSetOperator::RerollOnce, *sel)),
                    (RerollAndAdd, DExpr::Dice(dice)) => UnevalTree::RollSetOperation(DiceRoll { dice: *dice, results: Vec::new() }, RollSetOp(RollSetOperator::RerollAndAdd, *sel)),
                    (ExplodeOn,    DExpr::Dice(dice)) => UnevalTree::RollSetOperation(DiceRoll { dice: *dice, results: Vec::new() }, RollSetOp(RollSetOperator::ExplodeOn, *sel)),

                    (Reroll | RerollOnce | RerollAndAdd | ExplodeOn | Minimum | Maximum, _) => {
                        return Err(ValTreeError::DiceOnlySetOperation(*op));
                    }
                }
            }
        })
    }
}

impl Selection {
    pub(crate) fn select<'a>(
        &self,
        iter: impl IntoIterator<Item = &'a mut DieRoll>,
    ) -> Vec<&'a mut DieRoll> {
        use Selector::*;

        let iter = iter.into_iter();
        let Self(sel, Int(val)) = self;
        let val = *val;

        match sel {
            Literal => iter.filter(|r| r.last_roll() == val).collect(),
            GreaterThan => iter.filter(|r| r.last_roll() > val).collect(),
            LessThan => iter.filter(|r| r.last_roll() < val).collect(),
            ord_sel @ (Highest | Lowest) => {
                let mut ordered = iter.collect::<Vec<_>>();
                ordered.sort_by_key(|a| a.last_roll());

                match ord_sel {
                    Highest => {
                        let _ = ordered.drain(0..(ordered.len() - val as usize));
                        ordered
                    }
                    Lowest => {
                        let _ = ordered.drain((ordered.len() - val as usize)..);
                        ordered
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl RollSetOp {
    pub fn affects<'a>(&self, res: impl Iterator<Item = &'a mut DieRoll>) -> Vec<&'a mut DieRoll> {
        let Self(_, sel) = self;
        sel.select(res)
    }
}

impl UnevalTree {
    pub(crate) fn rolls_mut(
        &mut self,
    ) -> (
        Vec<&mut DiceRoll>,
        Vec<(&mut DiceRoll, &RollSetOp)>,
    ) {
        fn gather_rolls_mut<'a>(
            tree: &'a mut UnevalTree,
            roll_once: &mut Vec<&'a mut DiceRoll>,
            roll_many: &mut Vec<(&'a mut DiceRoll, &'a RollSetOp)>,
        ) {
            match tree {
                UnevalTree::Literal(_) => (),
                UnevalTree::Roll(roll) => roll_once.push(roll),
                UnevalTree::Labeled(tree, _)
                | UnevalTree::UnaryOperation(_, tree)
                | UnevalTree::ValSetOperation(tree, _) => {
                    gather_rolls_mut(tree, roll_once, roll_many)
                }
                UnevalTree::BinaryOperation(tree_l, _, tree_r) => {
                    gather_rolls_mut(tree_l, roll_once, roll_many);
                    gather_rolls_mut(tree_r, roll_once, roll_many);
                }
                UnevalTree::Set(trees) => {
                    trees
                        .iter_mut()
                        .for_each(|tree| gather_rolls_mut(tree, roll_once, roll_many));
                }
                UnevalTree::RollSetOperation(roll, op) => roll_many.push((roll, op)),
            }
        }

        let mut roll_once = Vec::new();
        let mut roll_many = Vec::new();

        gather_rolls_mut(self, &mut roll_once, &mut roll_many);
        (roll_once, roll_many)
    }

    pub(crate) fn finished(self) -> ValTree {
        match self {
            UnevalTree::Literal(lit) => ValTree::Literal(lit),
            UnevalTree::Roll(roll) => ValTree::Dice(roll),
            UnevalTree::UnaryOperation(op, tree) => {
                ValTree::UnaryOperation(op, Box::new(tree.finished()))
            }
            UnevalTree::Set(trees) => ValTree::Set(trees.into_iter().map(Self::finished).collect()),
            UnevalTree::ValSetOperation(tree, op) => {
                ValTree::SetOperation(Box::new(tree.finished()), op)
            }
            UnevalTree::BinaryOperation(lhs, op, rhs) => {
                ValTree::BinaryOperation(Box::new(lhs.finished()), op, Box::new(rhs.finished()))
            }
            UnevalTree::Labeled(tree, label) => ValTree::Labeled(Box::new(tree.finished()), label),
            UnevalTree::RollSetOperation(roll, _) => ValTree::Dice(roll),
        }
    }
}

type WorkingOut = f64;

impl ValTree {
    pub fn total(&self) -> i32 {
        let working_out = self._value();
        // Round to zero, so just get the fractional part

        let integral = working_out - working_out.fract();

        if integral > i32::MAX as f64 {
            panic!("TOO BIG!")
        }

        if integral < i32::MIN as f64 {
            panic!("TOO SMALL!");
        }

        integral as i32
    }

    fn _value(&self) -> WorkingOut {
        match self {
            ValTree::Labeled(tree, _) => tree._value(),

            // TODO: Better handling for these two primitives.
            ValTree::Literal(Literal::Int(int)) => WorkingOut::from(int.0),
            ValTree::Literal(Literal::Decimal(Decimal(f))) => WorkingOut::clone(f),

            ValTree::Dice(dice_roll) => dice_roll
                .results
                .iter()
                .map(|a| WorkingOut::from(a.value()))
                .sum(),
            ValTree::Set(trees) => trees.iter().map(|tree| tree._value()).sum(),

            ValTree::UnaryOperation(op, tree) => op.eval(tree._value()),
            ValTree::BinaryOperation(lhs, op, rhs) => op.eval(lhs._value(), rhs._value()),
            ValTree::SetOperation(set, op) => op.eval(set),
        }
    }
}

impl UnaryOperator {
    fn eval(&self, value: WorkingOut) -> WorkingOut {
        match self {
            UnaryOperator::Positive => value,
            UnaryOperator::Negative => -value,
        }
    }
}

impl BinaryOperator {
    fn compare(&self, ordering: Option<Ordering>) -> Option<bool> {
        use BinaryOperator::*; // Short names
        use std::cmp::Ordering::*; // Long names

        match (self, ordering?) {
            (Eq, Equal) => Some(true),
            (Eq, _) => Some(false),

            (GtE, Greater | Equal) => Some(true),
            (GtE, _) => Some(false),

            (LtE, Less | Equal) => Some(true),
            (LtE, _) => Some(false),

            (Gt, Greater) => Some(true),
            (Gt, _) => Some(false),

            (Lt, Less) => Some(true),
            (Lt, _) => Some(false),

            (NEq, Equal) => Some(false),
            (NEq, _) => Some(true),

            (_, _) => None,
        }
    }

    fn eval(&self, lhs: WorkingOut, rhs: WorkingOut) -> WorkingOut {
        match self {
            // TODO: Replace with checked {div, rem, ...}
            BinaryOperator::Add => lhs + rhs,
            BinaryOperator::Sub => lhs - rhs,

            BinaryOperator::Mul => lhs * rhs,
            BinaryOperator::Div => lhs / rhs,
            BinaryOperator::Rem => lhs % rhs,

            // TODO: Compare this with the python d20 library.
            BinaryOperator::IntDiv => (lhs / rhs).fract(),

            comparison @ (BinaryOperator::Eq
            | BinaryOperator::GtE
            | BinaryOperator::LtE
            | BinaryOperator::Gt
            | BinaryOperator::Lt
            | BinaryOperator::NEq) => match comparison.compare(lhs.partial_cmp(&rhs)) {
                Some(true) => 1.0,
                Some(false) | None => 0.0,
            },
        }
    }
}

impl ValSetOp {
    fn eval(&self, set: &ValTree) -> WorkingOut {
        use Selector::*;
        use ValSetOperator::*;

        let values = match set {
            ValTree::Set(trees) => {
                Box::new(trees.iter().map(|tree| tree._value())) as Box<dyn Iterator<Item = f64>>
            }
            ValTree::Dice(roll) => Box::new(roll.results.iter().map(|die| die.value() as f64)),
            ValTree::Labeled(tree, _) => return self.eval(tree.as_ref()),
            _ => unimplemented!("Should not have non-sets in a set op!"),
        };

        let Self(set_op, Selection(selector, Int(target))) = self;
        let target = WorkingOut::from(*target);

        match (set_op, selector) {
            (op @ (Keep | Drop), sel @ (GreaterThan | Literal | LessThan)) => {
                let comparison = |die: &f64| sel.is_ordering(die.total_cmp(&target)).unwrap();

                match op {
                    Keep => values.filter(|val| comparison(val)).sum(),
                    Drop => values.filter(|val| !comparison(val)).sum(),
                    _ => unreachable!(),
                }
            }

            (op @ (Keep | Drop), sel @ (Lowest | Highest)) => {
                let n = target as usize;
                let mut values = values.collect::<Vec<_>>();

                // TODO: Replace this with better static checking before evaluation!
                if n > values.len() {
                    panic!("Too many values to select!");
                }

                values.sort_by(|a, b| a.total_cmp(b));

                let range = match (op, sel) {
                    (Keep, Lowest) => 0..n,
                    (Drop, Lowest) => n..(values.len()),

                    (Keep, Highest) => (values.len() - n)..(values.len()),
                    (Drop, Highest) => 0..(values.len() - n),
                    _ => unreachable!(),
                };

                values.drain(range).sum()
            }

            // Literals only:
            //      Sets the minimum value of the die => max(die, target)
            (Minimum, Literal) => values.map(|val| val.max(target)).sum(),
            //      Sets the maximum value of the die => min(die, target)
            (Maximum, Literal) => values.map(|val| val.min(target)).sum(),

            // TODO: Fix the repr. of enum Selection to not make this necessary.
            (Maximum | Minimum, _) => unimplemented!(
                "Only literals can be used in conjunction with maximum or minimum set ops."
            ),
        }
    }
}

impl Selector {
    pub fn is_ordering(&self, ordering: Ordering) -> Option<bool> {
        use Selector as S;
        use std::cmp::Ordering as O;

        match (self, ordering) {
            (S::GreaterThan, O::Greater) => Some(true),
            (S::GreaterThan, O::Equal | O::Less) => Some(false),

            (S::Literal, O::Equal) => Some(true),
            (S::Literal, O::Greater | O::Less) => Some(false),

            (S::LessThan, O::Less) => Some(true),
            (S::LessThan, O::Greater | O::Equal) => Some(false),

            (S::Highest | S::Lowest, _) => None, // Not (absolute) selections
        }
    }
}

pub type ValTreeResult<T> = Result<T, ValTreeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValTreeError {
    DiceOnlySetOperation(SetOp),
    NonTerminating(SetOp),
}
