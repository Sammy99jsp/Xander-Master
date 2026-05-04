use crate::*;
use std::fmt::{Display, Write};

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOperator::Positive => f.write_str("+"),
            UnaryOperator::Negative => f.write_str("-"),
        }
    }
}

impl Display for SetOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", &self.0, &self.1, &self.1.1.0)
    }
}

impl Display for SetOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetOperator::Keep => f.write_str("k"),
            SetOperator::Drop => f.write_str("p"),
            SetOperator::Reroll => f.write_str("rr"),
            SetOperator::RerollOnce => f.write_str("ro"),
            SetOperator::RerollAndAdd => f.write_str("ra"),
            SetOperator::ExplodeOn => f.write_str("e"),
            SetOperator::Minimum => f.write_str("mi"),
            SetOperator::Maximum => f.write_str("ma"),
        }
    }
}

impl Display for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", &self.0, &self.1.0)
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Literal => f.write_str(""),
            Selector::Highest => f.write_str("h"),
            Selector::Lowest => f.write_str("l"),
            Selector::GreaterThan => f.write_str(">"),
            Selector::LessThan => f.write_str("<"),
        }
    }
}

impl Display for ValSetOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", &self.0, &self.1.0, &self.1.1.0)
    }
}

impl Display for ValSetOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValSetOperator::Keep => f.write_str("k"),
            ValSetOperator::Drop => f.write_str("p"),
            ValSetOperator::Minimum => f.write_str("mi"),
            ValSetOperator::Maximum => f.write_str("ma"),
        }
    }
}

impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOperator::Mul => f.write_str("*"),
            BinaryOperator::Div => f.write_str("/"),
            BinaryOperator::IntDiv => f.write_str("//"),
            BinaryOperator::Rem => f.write_str("%"),
            BinaryOperator::Add => f.write_str("+"),
            BinaryOperator::Sub => f.write_str("-"),
            BinaryOperator::Eq => f.write_str("=="),
            BinaryOperator::GtE => f.write_str(">="),
            BinaryOperator::LtE => f.write_str("<="),
            BinaryOperator::Gt => f.write_str(">"),
            BinaryOperator::Lt => f.write_str("<"),
            BinaryOperator::NEq => f.write_str("!="),
        }
    }
}

pub struct ShowOptions {}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum Precedence {
    Cmp,
    Add,
    Mul,
}

impl Precedence {
    pub const fn op(op: &BinaryOperator) -> Self {
        match op {
            BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::IntDiv
            | BinaryOperator::Rem => Precedence::Mul,

            BinaryOperator::Add | BinaryOperator::Sub => Precedence::Add,

            BinaryOperator::Gt
            | BinaryOperator::Lt
            | BinaryOperator::Eq
            | BinaryOperator::GtE
            | BinaryOperator::LtE
            | BinaryOperator::NEq => Precedence::Cmp,
        }
    }
}

impl BinaryOperator {
    fn opposes(&self, inner: &Self) -> bool {
        self.precedence() == inner.precedence() && self != inner
    }
}

pub fn parenthesize_if_necessary<T>(
    show: fn(&mut String, &T) -> std::fmt::Result,
    out: &mut String,
    op: &BinaryOperator,
    sub_op: &BinaryOperator,
    expr: &T,
    is_right: bool,
) -> std::fmt::Result {
    if sub_op > op || (is_right && op.opposes(sub_op)) {
        out.push('(');
        show(out, expr)?;
        out.push(')');
    } else {
        show(out, expr)?;
    }

    Ok(())
}

macro_rules! show_match {
    ($ty: ident, $expr: expr, $out: ident) => {
        match $expr {
            $ty::Literal(Literal::Decimal(Decimal(literal))) => {
                $out.write_fmt(core::format_args!("{literal}"))
            }
            $ty::Literal(Literal::Int(Int(literal))) => {
                $out.write_fmt(core::format_args!("{literal}"))
            }
            $ty::Labeled(Labeled(box expr, _)) => _show($out, expr),
            $ty::Set(exprs) => {
                $out.push('[');
                for (last, expr) in exprs
                    .iter()
                    .enumerate()
                    .map(|(i, expr)| (i == exprs.len() - 1, expr))
                {
                    _show($out, expr)?;
                    if !last {
                        $out.push_str(", ");
                    }
                }
                $out.push(']');
                Ok(())
            }
            $ty::UnaryOperation(unary_operator, expr @ box $ty::BinaryOperation(..)) => {
                $out.write_fmt(core::format_args!("{unary_operator}("))?;
                _show($out, expr)?;
                $out.push(')');
                Ok(())
            }
            $ty::UnaryOperation(unary_operator, box expr) => {
                $out.write_fmt(core::format_args!("{unary_operator}"))?;
                _show($out, expr)
            }
            $ty::SetOperation(expr @ box $ty::UnaryOperation(..), set_op) => {
                $out.push('(');
                _show($out, expr)?;
                $out.push(')');
                $out.write_fmt(core::format_args!("{set_op}"))
            }
            $ty::SetOperation(expr @ box $ty::BinaryOperation(..), set_op) => {
                $out.push('(');
                _show($out, expr)?;
                $out.push(')');
                $out.write_fmt(core::format_args!("{set_op}"))
            }
            $ty::SetOperation(box expr, set_op) => {
                _show($out, expr)?;
                $out.write_fmt(core::format_args!("{set_op}"))
            }
            $ty::BinaryOperation(
                lhs @ box $ty::BinaryOperation(_, l_op, _),
                op,
                rhs @ box $ty::BinaryOperation(_, r_op, _),
            ) => {
                parenthesize_if_necessary(_show, $out, op, l_op, lhs, false)?;
                $out.write_fmt(core::format_args!(" {op} "))?;
                parenthesize_if_necessary(_show, $out, op, r_op, rhs, true)
            }
            $ty::BinaryOperation(box lhs, op, rhs @ box $ty::BinaryOperation(_, r_op, _)) => {
                _show($out, lhs)?;
                $out.write_fmt(core::format_args!(" {op} "))?;
                parenthesize_if_necessary(_show, $out, op, r_op, rhs, true)
            }
            $ty::BinaryOperation(lhs @ box $ty::BinaryOperation(_, l_op, _), op, box rhs) => {
                parenthesize_if_necessary(_show, $out, op, l_op, lhs, false)?;
                $out.write_fmt(core::format_args!(" {op} "))?;
                _show($out, rhs)
            }
            $ty::BinaryOperation(box lhs, op, box rhs) => {
                _show($out, lhs)?;
                $out.write_fmt(core::format_args!(" {op} "))?;
                _show($out, rhs)
            }
            _ => unimplemented!("Should cover the dice case separately..."),
        }
    };
}

impl DExpr {
    pub fn show(&self) -> String {
        pub fn _show(out: &mut String, expr: &DExpr) -> std::fmt::Result {
            match expr {
                DExpr::Dice(Dice {
                    qty: Some(Int(qty)),
                    sides: Int(sides),
                }) => return write!(out, "{qty}d{sides}"),
                DExpr::Dice(Dice {
                    qty: None,
                    sides: Int(sides),
                }) => return write!(out, "d{sides}"),
                _ => (),
            }

            show_match!(DExpr, expr, out)
        }
        let mut out = String::new();
        _show(&mut out, self).unwrap();
        out
    }
}

fn strikethrough(f: &mut std::fmt::Formatter<'_>, a: &impl Display) -> std::fmt::Result {
    write!(f, "\u{0337}{a}")
}

impl Display for DieRoll {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DieRoll::Normal(roll) => write!(f, "{roll}"),
            DieRoll::Replace {
                original, box new, ..
            } => {
                strikethrough(f, original)?;
                f.write_str(", ")?;
                new.fmt(f)
            }
            DieRoll::InAddition {
                original, box new, ..
            } => {
                write!(f, "{original}, ")?;
                new.fmt(f)
            }
        }
    }
}

impl ValTree {
    pub fn show(&self) -> String {
        pub fn _show(out: &mut String, expr: &ValTree) -> std::fmt::Result {
            match expr {
                ValTree::Dice(DiceRoll { results, .. }) => {
                    out.push('[');
                    for (last, result) in results
                        .iter()
                        .enumerate()
                        .map(|(i, roll)| (i == results.len() - 1, roll))
                    {
                        write!(out, "{result}")?;

                        if !last {
                            out.push_str(", ");
                        }
                    }
                    out.push(']');

                    return Ok(());
                }
                ValTree::Set(expr) if let [dice @ ValTree::Dice(_)] = expr.as_slice() => {
                    return _show(out, dice);
                }
                _ => (),
            }

            show_match!(ValTree, expr, out)
        }
        let mut out = String::new();
        _show(&mut out, self).unwrap();
        out
    }
}

impl Display for DExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.show())
    }
}

impl Display for ValTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.show())
    }
}

#[test]
fn test_display() {
    let expr = DExpr::SetOperation(
        Box::new(DExpr::Dice(Dice {
            qty: Some(Int(2)),
            sides: Int(20),
        })),
        SetOp(SetOperator::Keep, Selection(Selector::Highest, Int(1))),
    ) + 4;

    let rng = crate::provider::local_rng::LocalRng::new();
    let res = rng.roll(&expr).unwrap().into_inner().unwrap();
    println!("{} == {}", res.show(), res.total())
}
