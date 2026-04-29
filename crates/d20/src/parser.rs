use chumsky::{prelude::*, text::digits};

use crate::SetOp;

/* TODO: Make it return an actually nice Result with line:col information. */
pub fn parse(input: &str) -> Option<crate::DExpr> {
    crate::DExpr::parser().parse(input).into_output()
}

impl crate::DExpr {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> {
        recursive(|expr| {
            let atom = choice((
                crate::Dice::parser().map(Self::Dice),
                crate::Literal::parser().map(Self::Literal),
                // (<expr>, <expr>, <expr>, ...)
                expr.clone()
                    .separated_by(just(",").padded())
                    .collect::<Vec<_>>()
                    .delimited_by(just("(").padded(), just(")").padded())
                    .map(Self::Set),
            ));

            let unary = crate::UnaryOperator::parser()
                .repeated()
                .foldr(atom, |op, expr| {
                    crate::DExpr::UnaryOperation(op, Box::new(expr))
                });

            let set_op = unary
                .then(SetOp::parser().or_not())
                .map(|(expr, set_op)| match set_op {
                    Some(set_op) => crate::DExpr::SetOperation(Box::new(expr), set_op),
                    None => expr,
                });

            let product = set_op.clone().foldl(
                crate::BinaryOperator::parser()
                    .filter(|op| op.precedence() == 0)
                    .then(set_op)
                    .repeated(),
                |lhs, (op, rhs)| crate::DExpr::BinaryOperation(Box::new(lhs), op, Box::new(rhs)),
            );

            let sum = product.clone().foldl(
                crate::BinaryOperator::parser()
                    .filter(|op| op.precedence() == 1)
                    .then(product)
                    .repeated(),
                |lhs, (op, rhs)| crate::DExpr::BinaryOperation(Box::new(lhs), op, Box::new(rhs)),
            );

            #[allow(clippy::let_and_return)]
            let logic = sum.clone().foldl(
                crate::BinaryOperator::parser()
                    .filter(|op| op.precedence() == 2)
                    .then(sum)
                    .repeated(),
                |lhs, (op, rhs)| crate::DExpr::BinaryOperation(Box::new(lhs), op, Box::new(rhs)),
            );

            logic
        })
    }
}

impl crate::Dice {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        crate::Int::parser()
            .or_not()
            .then_ignore(just("d"))
            .then(crate::Int::parser())
            .map(|(qty, sides)| Self { qty, sides })
    }
}

impl crate::Literal {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        choice((
            crate::Decimal::parser().map(Self::Decimal),
            crate::Int::parser().map(Self::Int),
        ))
    }
}

impl crate::SetOp {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        crate::SetOperator::parser()
            .then(crate::Selection::parser())
            .map(|(a, b)| Self(a, b))
    }
}

impl crate::Selection {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        crate::Selector::parser()
            .then(crate::Int::parser())
            .map(|(a, b)| Self(a, b))
    }
}

impl crate::Decimal {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        digits(10)
            .then(just(".").ignored())
            .then(digits(10))
            .to_slice()
            .from_str::<f64>()
            .unwrapped()
            .map(Self)
    }
}

impl crate::Int {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        digits(10).to_slice().from_str().unwrapped().map(Self)
    }
}

impl crate::SetOperator {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        choice((
            just("k").to(Self::Keep),
            just("p").to(Self::Drop),
            just("rr").to(Self::Reroll),
            just("ro").to(Self::RerollOnce),
            just("ra").to(Self::RerollAndAdd),
            just("e").to(Self::ExplodeOn),
            just("mi").to(Self::Minimum),
            just("ma").to(Self::Maximum),
        ))
    }
}

impl crate::Selector {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        choice((
            just("h").to(Self::Highest),
            just("l").to(Self::Lowest),
            just(">").to(Self::GreaterThan),
            just("<").to(Self::LessThan),
            empty().to(Self::Literal),
        ))
    }
}

impl crate::UnaryOperator {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        choice((just("+").to(Self::Positive), just("-").to(Self::Negative)))
    }
}

impl crate::BinaryOperator {
    pub fn parser<'a>() -> impl chumsky::Parser<'a, &'a str, Self> + Clone {
        choice((
            just("*").to(Self::Mul),
            just("//").to(Self::IntDiv),
            just("/").to(Self::Div),
            just("%").to(Self::Rem),
            just("+").to(Self::Add),
            just("-").to(Self::Sub),
            just("==").to(Self::Eq),
            just("!=").to(Self::NEq),
            just(">=").to(Self::GtE),
            just(">").to(Self::Gt),
            just("<=").to(Self::LtE),
            just("<").to(Self::Lt),
        ))
    }
}

#[cfg(test)]
mod test {
    use chumsky::Parser;

    /* TODO: Write some more unit tests here. */
    #[test]
    fn a() {
        let parser = crate::DExpr::parser();

        let _ = parser.parse("1+d8+4d4p<1").into_result();
    }
}
