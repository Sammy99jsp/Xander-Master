use std::{
    collections::{
        BTreeMap,
        btree_map::{Entry, IterMut},
    },
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Sub, SubAssign},
    rc::{Rc, Weak},
};

use crate::engine::{
    game::combat::{Attack, Combatant},
    io::roller::{DiceRollerError, Roller},
};

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Damage<T> {
    parts: BTreeMap<DamageType, T>,
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
#[rkyv(derive(PartialOrd, PartialEq, Ord, Eq))]
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

#[derive(Debug, Clone)]
pub enum DamageSourceType {
    Attack(Weak<Attack>),
}

#[derive(Debug)]
pub struct DamageSource {
    pub from: Option<Weak<Combatant>>,
    pub ty: DamageSourceType,
}

impl<T> Damage<T> {
    pub const fn new() -> Self {
        Self {
            parts: BTreeMap::new(),
        }
    }

    pub fn types(&self) -> usize {
        self.parts.len()
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

    pub fn filter<F>(self, mut f: F) -> Damage<T>
    where
        F: for<'a> FnMut(DamageType, &'a T) -> bool,
    {
        Damage {
            parts: self
                .parts
                .into_iter()
                .filter(|(ty, expr)| f(*ty, expr))
                .collect(),
        }
    }

    pub fn filter_map<U, F>(self, mut f: F) -> Damage<U>
    where
        F: FnMut(DamageType, T) -> Option<U>,
    {
        Damage {
            parts: self
                .parts
                .into_iter()
                .filter_map(|(ty, expr)| f(ty, expr).map(|expr| (ty, expr)))
                .collect(),
        }
    }

    pub fn as_ref(&self) -> Damage<&'_ T> {
        Damage {
            parts: self.parts.iter().map(|(ty, dexpr)| (*ty, dexpr)).collect(),
        }
    }

    pub fn as_mut(&mut self) -> Damage<&'_ mut T> {
        Damage {
            parts: self
                .parts
                .iter_mut()
                .map(|(ty, dexpr)| (*ty, dexpr))
                .collect(),
        }
    }

    pub fn map<U, F>(self, mut f: F) -> Damage<U>
    where
        F: FnMut(DamageType, T) -> U,
    {
        Damage {
            parts: self
                .parts
                .into_iter()
                .map(|(ty, value)| (ty, f(ty, value)))
                .collect(),
        }
    }

    pub fn for_each<F>(self, mut f: F)
    where
        F: FnMut(DamageType, T),
    {
        self.parts.into_iter().for_each(|(ty, expr)| f(ty, expr));
    }

    #[inline]
    pub fn get_mut(&mut self, ty: DamageType) -> Option<&mut T> {
        self.parts.get_mut(&ty)
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, DamageType, T> {
        self.parts.iter_mut()
    }
}

impl<T> FromIterator<(DamageType, T)> for Damage<T>
where
    T: AddAssign,
{
    fn from_iter<Iter>(iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = (DamageType, T)>,
    {
        let mut out = Self::new();
        for (ty, value) in iter.into_iter() {
            match out.parts.entry(ty) {
                Entry::Vacant(empty) => {
                    empty.insert(value);
                }
                Entry::Occupied(mut occupied) => {
                    *occupied.get_mut() += value;
                }
            }
        }

        out
    }
}

impl<T> IntoIterator for Damage<T> {
    type Item = (DamageType, T);

    type IntoIter = std::collections::btree_map::IntoIter<DamageType, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.parts.into_iter()
    }
}

impl<T> Default for Damage<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Specialized methods

impl Damage<d20::DExpr> {
    pub async fn roll<R>(&self, roller: &R) -> Result<Damage<d20::ValTree>, DiceRollerError>
    where
        R: Roller + ?Sized,
    {
        let sum = {
            let mut iter = self.parts.iter();

            let Some(first) = iter.next() else {
                return Ok(Damage::new());
            };

            let first = first.1.clone().label(Rc::new(*first.0));

            iter.fold(first, |sum, (ty, expr)| {
                sum + expr.clone().label(Rc::new(*ty))
            })
        };

        let mut sum = roller.roll(&sum).await?;

        let mut output = Damage::new();

        loop {
            let d20::ValTree::BinaryOperation(
                lhs,
                d20::BinaryOperator::Add,
                box d20::ValTree::Labeled(d20::Labeled(box rhs, d20::Label(label))),
            ) = sum
            else {
                break;
            };

            let ty: DamageType = unsafe { *label.unwrap().downcast_rc().unwrap_unchecked() };
            output.parts.insert(ty, rhs);

            sum = *lhs;
        }

        let d20::ValTree::Labeled(d20::Labeled(box lhs, d20::Label(label))) = sum else {
            unreachable!("Should be single item at this stage!")
        };

        let ty: DamageType = unsafe { *label.unwrap().downcast_rc().unwrap_unchecked() };
        output.parts.insert(ty, lhs);

        Ok(output)
    }
}

// Math
const ADD: fn(i32, i32) -> i32 = i32::saturating_add;

impl<T> Damage<T>
where
    T: Add<T, Output = T>,
    T: Default + Clone,
{
    pub fn sum(&self) -> T {
        if self.parts.is_empty() {
            return T::default();
        }

        let mut iter = self.parts.iter();
        let first = iter.next().unwrap().1.clone();
        iter.fold(first, |a, (_, b)| a + b.clone())
    }
}

impl Damage<d20::ValTree> {
    pub fn subtotal(&self) -> Damage<i32> {
        self.as_ref().map(|_, expr| expr.total())
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

// FORMATTING
impl Display for DamageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl<T: Display> Display for Damage<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref()
            .into_iter()
            .enumerate()
            .map(|(i, (k, v))| (i == self.types() - 1, k, v))
            .try_for_each(|(last, ty, value)| {
                write!(f, "{value} {ty}")?;
                if !last {
                    f.write_str(" + ")?;
                }

                Ok(())
            })
    }
}

// UI

pub mod ui {
    use xander_runtime::{register, ui};

    use super::DamageType;

    impl ui::Ui for DamageType {}
    register!(DamageType, register(Identity("HEALTH::DAMAGE_TYPE")));
}
