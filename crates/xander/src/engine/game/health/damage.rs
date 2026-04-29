use std::{
    collections::{
        BTreeMap,
        btree_map::{Entry, IterMut},
    },
    ops::{Add, AddAssign, Sub, SubAssign},
    rc::Rc,
};

#[derive(Debug, Clone)]
pub struct Damage<T> {
    parts: BTreeMap<DamageType, T>,
}

impl<T> Damage<T> {
    pub const fn new() -> Self {
        Self {
            parts: BTreeMap::new(),
        }
    }

    pub fn of(ty: DamageType, amount: T) -> Self {
        Self {
            parts: {
                let mut parts = BTreeMap::new();
                parts.insert(ty, amount);
                parts
            },
        }
    }

    pub fn map<U, F>(&self, mut f: F) -> Damage<U>
    where
        F: for<'a> FnMut(&'a T) -> U,
    {
        Damage {
            parts: self
                .parts
                .iter()
                .map(|(ty, value)| (*ty, f(value)))
                .collect(),
        }
    }

    #[inline]
    pub fn get_mut(&mut self, ty: DamageType) -> Option<&mut T> {
        self.parts.get_mut(&ty)
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, DamageType, T> {
        self.parts.iter_mut()
    }
}

impl<T> Default for Damage<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lighting,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

// Specialized methods

impl Damage<d20::DExpr> {
    pub async fn roll<R: d20::DiceRoller>(&self, roller: R) -> Damage<d20::ValTree> {
        let sum = {
            let mut iter = self.parts.iter();

            let Some(first) = iter.next() else {
                return Damage::new();
            };

            let first = first.1.clone().label(Rc::new(*first.0));

            iter.fold(first, |sum, (ty, expr)| {
                sum + expr.clone().label(Rc::new(*ty))
            })
        };

        let Ok(sum) = roller.roll(&sum) else { todo!() };

        let Ok(mut sum) = sum.await else { todo!() };

        let mut output = Damage::new();

        loop {
            let d20::ValTree::BinaryOperation(
                lhs,
                d20::BinaryOperator::Add,
                box d20::ValTree::Labeled(box rhs, d20::Label(label)),
            ) = sum
            else {
                break;
            };

            let ty: DamageType = unsafe { *label.downcast_rc().unwrap_unchecked() };
            output.parts.insert(ty, rhs);

            sum = *lhs;
        }

        let d20::ValTree::Labeled(box lhs, d20::Label(label)) = sum else {
            unreachable!("Should be single item at this stage!")
        };

        let ty: DamageType = unsafe { *label.downcast_rc().unwrap_unchecked() };
        output.parts.insert(ty, lhs);

        output
    }
}

// Math
const ADD: fn(i32, i32) -> i32 = i32::saturating_add;

impl Damage<i32> {
    pub fn sum(&self) -> i32 {
        self.parts.values().copied().fold(0, ADD)
    }
}

impl Damage<d20::ValTree> {
    pub fn subtotal(&self) -> Damage<i32> {
        self.map(|expr| expr.total())
    }

    pub fn total(&self) -> i32 {
        self.parts
            .values()
            .fold(0, |total, expr| ADD(total, expr.total()))
    }
}

impl<T> Damage<T> {
    #[inline]
    fn in_place_binary_op<Rhs>(self, rhs: Damage<Rhs>, f: fn(&mut T, Rhs)) -> Self
    where
        T: From<Rhs>,
    {
        let mut output = self;

        for (ty, rhs) in rhs.parts {
            match output.parts.entry(ty) {
                Entry::Vacant(vacant) => {
                    vacant.insert(T::from(rhs));
                }
                Entry::Occupied(mut occupied) => f(occupied.get_mut(), rhs),
            }
        }

        output
    }
}

impl<Lhs, Rhs> Add<Damage<Rhs>> for Damage<Lhs>
where
    Lhs: AddAssign<Rhs>,
    Lhs: From<Rhs>,
{
    type Output = Damage<Lhs>;

    fn add(self, rhs: Damage<Rhs>) -> Self::Output {
        self.in_place_binary_op(rhs, |lhs, rhs| *lhs += rhs)
    }
}

impl<Lhs, Rhs> Sub<Damage<Rhs>> for Damage<Lhs>
where
    Lhs: SubAssign<Rhs>,
    Lhs: From<Rhs>,
{
    type Output = Damage<Lhs>;

    fn sub(self, rhs: Damage<Rhs>) -> Self::Output {
        self.in_place_binary_op(rhs, |lhs, rhs| *lhs -= rhs)
    }
}

// UI

pub mod ui {
    use xander_runtime::ui;

    use super::DamageType;

    impl ui::Ui for DamageType {}
}
