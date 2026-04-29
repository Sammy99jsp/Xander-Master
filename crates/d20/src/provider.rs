use std::ops::{ControlFlow, DerefMut};

use crate::{
    Dice, DiceRoll, DieRoll, RollSetOp, RollSetOperator, UnevalTree, ValTree, ValTreeError,
};

/// Wrapper to allow different environments (web UI, TUI, etc.) to govern how dice are rolled.
///
/// For most use cases, you will not need to encounter this trait.
///
/// This trait is handy for certain use cases
pub trait DiceRoller: Sized {
    type Fut<T>: DiceFuture<T>;
    type RollErr;

    fn map_fut<T, U>(fut: Self::Fut<T>, func: impl FnOnce(T) -> U) -> Self::Fut<U>;
    fn ok_then_fut<T, U>(
        fut: Self::Fut<Result<T, Self::RollErr>>,
        func: impl FnOnce(T) -> Self::Fut<U>,
    ) -> Self::Fut<Result<U, Self::RollErr>>;
    fn flat_fut<T>(fut: Self::Fut<Self::Fut<T>>) -> Self::Fut<T>;

    fn fold_until_fut<T, U>(
        fut: Self::Fut<T>,
        step: impl FnMut(T) -> ControlFlow<U, Self::Fut<T>>,
    ) -> Self::Fut<U>;

    fn roll_dice<'d>(
        &self,
        dice: impl IntoIterator<Item = &'d Dice>,
    ) -> Self::Fut<Result<Vec<Vec<u32>>, Self::RollErr>>;

    fn roll(
        &self,
        expr: &crate::DExpr,
    ) -> Result<Self::Fut<Result<ValTree, Self::RollErr>>, ValTreeError> {
        let tree = expr.to_uneval()?;
        Ok(tree.eval_with(self))
    }
}

pub type RollerResult<T, R> = Result<T, <R as DiceRoller>::RollErr>;

pub trait DiceFuture<T>: IntoFuture<Output = T> + Sized {
    fn map<R, U>(self, func: impl FnOnce(T) -> U) -> <R as DiceRoller>::Fut<U>
    where
        R: DiceRoller<Fut<T> = Self>,
    {
        R::map_fut(self, func)
    }

    fn and_then<R, U>(
        self,
        func: impl FnOnce(T) -> <R as DiceRoller>::Fut<U>,
    ) -> <R as DiceRoller>::Fut<U>
    where
        R: DiceRoller<Fut<T> = Self>,
    {
        R::flat_fut::<U>(R::map_fut(self, func))
    }

    fn fold_until<R, U>(
        self,
        step: impl FnMut(T) -> ControlFlow<U, R::Fut<T>>,
    ) -> <R as DiceRoller>::Fut<U>
    where
        R: DiceRoller<Fut<T> = Self>,
    {
        R::fold_until_fut(self, step)
    }
}

impl<Fut, T> DiceFuture<T> for Fut where Fut: IntoFuture<Output = T> {}

pub trait TryDiceFut<T, E>: DiceFuture<Result<T, E>> {
    fn ok_then<R, U>(self, func: impl FnOnce(T) -> R::Fut<U>) -> R::Fut<Result<U, R::RollErr>>
    where
        R: DiceRoller<RollErr = E, Fut<Result<T, E>> = Self>,
    {
        R::ok_then_fut::<T, U>(self, func)
    }
}

impl<Fut, T, E> TryDiceFut<T, E> for Fut where Fut: DiceFuture<Result<T, E>> {}

impl UnevalTree {
    #[expect(clippy::type_complexity)]
    pub fn eval_with<R: DiceRoller>(mut self, roller: &R) -> R::Fut<Result<ValTree, R::RollErr>> {
        let (roll_once, roll_many) = self.rolls_mut();
        let (mut roll_many, mut roll_twice) =
            roll_many
                .into_iter()
                .partition::<Vec<_>, _>(|(_, RollSetOp(op, _))| match op {
                    RollSetOperator::Reroll => true,
                    RollSetOperator::RerollOnce => false,
                    RollSetOperator::RerollAndAdd => false,
                    RollSetOperator::ExplodeOn => true,
                });

        // 1A. Roll everything initially.
        let (dice, roll_results): (Vec<&_>, Vec<_>) = {
            roll_once
                .into_iter() // Drops roll_once
                .chain(
                    roll_twice
                        .iter_mut()
                        .chain(roll_many.iter_mut())
                        .map(|(roll, _)| roll.deref_mut()),
                )
                .map(|roll| (&roll.dice, &mut roll.results))
                .unzip()
        };

        roller
            .roll_dice(dice)
            .map::<R, _>(move |results| -> RollerResult<_, R> {
                // 1B. Set the corresponding results for all dice rolls.
                roll_results
                    .into_iter()
                    .zip(results?)
                    .for_each(|(to_set, result)| {
                        to_set.extend(result.into_iter().map(DieRoll::Normal))
                    });
                Ok(())
            })
            .map::<R, _>(move |res| -> RollerResult<_, R> {
                let () = res?;

                // 2A. Keep the roll_{twice, many} ops that need to be re-rolled.
                let rerolls_for = |arr: Vec<_>| -> Vec<_> {
                    arr.into_iter()
                        .filter_map(|(rolls, op): (&mut DiceRoll, &RollSetOp)| {
                            let rerolls = op.affects(rolls.results.iter_mut());
                            if rerolls.is_empty() {
                                None
                            } else {
                                Some((rolls.dice, rerolls, op))
                            }
                        })
                        .collect()
                };

                let roll_twice = rerolls_for(roll_twice);
                let roll_many = rerolls_for(roll_many);
                Ok((roll_twice, roll_many))
            })
            .ok_then::<R, _>(
                |(mut roll_twice, mut roll_many): (Vec<_>, Vec<_>)| -> R::Fut<_> {
                    // 2B. Roll re-rolls
                    let (dice, to_sets): (Vec<_>, Vec<_>) = roll_twice
                        .iter_mut()
                        .chain(roll_many.iter_mut())
                        .map(
                            |(dice, to_reroll, op): &mut (Dice, Vec<&mut DieRoll>, &RollSetOp)| {
                                let mut dice = *dice;
                                dice.qty = Some(crate::Int(to_reroll.len() as u32)); // to_reroll.len() > 0

                                (dice, (to_reroll, op))
                            },
                        )
                        .unzip();

                    // 2C. Set the corresponding results for all re-rolls.
                    roller
                        .roll_dice(dice.iter())
                        .map::<R, _>(|reroll_results| -> RollerResult<_, R> {
                            to_sets.into_iter().zip(reroll_results?).for_each(
                                |((to_set, op), reroll_result)| {
                                    to_set.iter_mut().zip(reroll_result).for_each(
                                        |(to_set, new_roll)| {
                                            to_set.update_with(**op, new_roll);
                                        },
                                    );
                                },
                            );

                            Ok(())
                        })
                        .map::<R, _>(move |res| -> RollerResult<_, R> {
                            let () = res?;
                            Ok(roll_many)
                        })
                },
            )
            .map::<R, _>(Result::flatten)
            .fold_until::<R, _>(
                |roll_many| -> ControlFlow<Result<(), R::RollErr>, R::Fut<Result<_, R::RollErr>>> {
                    let mut roll_many = match roll_many {
                        Ok(ok) => ok,
                        Err(err) => return ControlFlow::Break(Err(err)),
                    };

                    let (dice, to_sets): (Vec<_>, Vec<_>) = roll_many
                        .iter_mut()
                        .filter_map(|(dice, rolls, op)| {
                            let rerolls = op.affects(rolls.iter_mut().map(DerefMut::deref_mut));
                            let mut dice = *dice;
                            dice.qty = Some(crate::Int(rerolls.len() as u32));

                            if rerolls.is_empty() {
                                None
                            } else {
                                Some((dice, (rerolls, op)))
                            }
                        })
                        .unzip();

                    if to_sets.is_empty() {
                        return ControlFlow::Break(Ok(()));
                    }

                    ControlFlow::Continue(
                        roller
                            .roll_dice(dice.iter())
                            .map::<R, _>(|reroll_results| -> RollerResult<_, R> {
                                to_sets.into_iter().zip(reroll_results?).for_each(
                                    |((mut to_set, op), reroll_result)| {
                                        to_set.iter_mut().zip(reroll_result).for_each(
                                            |(to_set, new_roll)| {
                                                to_set.update_with(**op, new_roll);
                                            },
                                        );
                                    },
                                );

                                Ok(())
                            })
                            .map::<R, _>(move |res| -> RollerResult<_, R> {
                                let () = res?;
                                Ok(roll_many)
                            }),
                    )
                },
            )
            .map::<R, _>(|res| -> RollerResult<_, R> {
                let () = res?;
                Ok(self.finished())
            })
    }
}

#[cfg(feature = "rand")]
pub mod local_rng {
    use rand::{
        SeedableRng,
        distr::{Distribution, Uniform},
        rngs::StdRng,
    };
    use std::{
        cell::RefCell,
        future::{Ready, ready},
        sync::OnceLock,
    };

    use super::DiceRoller;
    use crate::{Dice, Int};

    /// Thread-local RNG.
    ///
    /// Internally uses [rand::rngs::StdRng].
    #[derive(Debug)]
    pub struct LocalRng(u64, OnceLock<RefCell<StdRng>>);

    impl LocalRng {
        /// Create a new [LocalRng] with a given seed.
        pub const fn new(seed: u64) -> Self {
            Self(seed, OnceLock::new())
        }
    }

    impl Default for LocalRng {
        fn default() -> Self {
            Self::new(Default::default())
        }
    }

    impl DiceRoller for LocalRng {
        type Fut<T> = Ready<T>;
        type RollErr = !;

        fn map_fut<T, U>(fut: Self::Fut<T>, func: impl FnOnce(T) -> U) -> Self::Fut<U> {
            let t = fut.into_inner();
            ready(func(t))
        }

        fn flat_fut<T>(fut: Self::Fut<Self::Fut<T>>) -> Self::Fut<T> {
            fut.into_inner()
        }

        fn ok_then_fut<T, U>(
            fut: Self::Fut<Result<T, Self::RollErr>>,
            func: impl FnOnce(T) -> Self::Fut<U>,
        ) -> Self::Fut<Result<U, Self::RollErr>> {
            match fut.into_inner() {
                Ok(ok) => ready(Ok(func(ok).into_inner())),
            }
        }

        fn roll_dice<'d>(
            &self,
            dice: impl IntoIterator<Item = &'d Dice>,
        ) -> Ready<Result<Vec<Vec<u32>>, Self::RollErr>> {
            let mut rng = self
                .1
                .get_or_init(|| RefCell::new(StdRng::seed_from_u64(self.0)))
                .borrow_mut();

            ready(Ok(dice
                .into_iter()
                .map(
                    move |dice @ &Dice {
                              sides: Int(sides), ..
                          }| {
                        let dist = Uniform::new_inclusive(1, sides).unwrap();

                        dist.sample_iter(&mut rng)
                            .take(dice.qty() as usize)
                            .collect::<Vec<_>>()
                    },
                )
                .collect::<Vec<_>>()))
        }

        fn fold_until_fut<T, U>(
            fut: Self::Fut<T>,
            mut step: impl FnMut(T) -> std::ops::ControlFlow<U, Self::Fut<T>>,
        ) -> Self::Fut<U> {
            let mut current = fut;
            loop {
                match step(current.into_inner()) {
                    std::ops::ControlFlow::Continue(res) => {
                        current = res;
                    }
                    std::ops::ControlFlow::Break(end) => return ready(end),
                }
            }
        }
    }

    #[cfg(all(test, feature = "rand"))]
    mod tests {
        use crate::{DiceRoller, LocalRng, parse};

        #[test]
        fn test_rng() {
            let four_d6_kl_2 = parse("2d6ro<3").unwrap();
            let rng = LocalRng::new(367324233);

            let a = rng.roll(&four_d6_kl_2).unwrap().into_inner().unwrap();
            println!("{a:#?}")
        }
    }
}
