//! Resistances, Immunities, and Vulnerabilities
//!
//! If you have a better name for this, please tell me...

use std::rc::Rc;

use d20::ValTree;
use dynx::{Namespace, dynx::Single};
use xander_runtime::{
    DynWeak, Lived, dependently_alive,
    lived::{LivedList, LivedSerializable, OptionalDependency},
    register,
};

use crate::engine::game::health::{Damage, DamageType};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct RIV {
    pub resistances: LivedList<Rc<Resistance>>,
    pub immunities: LivedList<Rc<Immunity>>,
    pub vulnerabilities: LivedList<Rc<Vulnerability>>,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Resistance(pub DamageEffect);
dependently_alive!(Resistance, 0);

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Vulnerability(pub DamageEffect);
dependently_alive!(Vulnerability, 0);

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct DamageEffect {
    pub dep: OptionalDependency<DynWeak<dyn LivedSerializable>>,
    pub to: DamageFilter,
}
dependently_alive!(DamageEffect, dep);

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum DamageFilter {
    Type(DamageType),
    Filter(Single<dyn HealthFilter>),
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Immunity {
    pub dep: OptionalDependency<DynWeak<dyn LivedSerializable>>,
    pub to: ImmunityTarget,
}
dependently_alive!(Immunity, dep);

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ImmunityTarget {
    Damage(DamageFilter),
    Condition(()),
}

#[Namespace("HEALTH_FILTER" @ NS, derive(Singleton))]
pub trait HealthFilter: std::fmt::Debug + Lived {
    fn filter<'b>(&self, damage: &'b mut Damage<ValTree>) -> Option<&'b mut ValTree>;
}

impl RIV {
    pub fn new() -> Self {
        Self {
            resistances: LivedList::new(),
            immunities: LivedList::new(),
            vulnerabilities: LivedList::new(),
        }
    }

    pub fn apply_to_damage(&self, damage: &mut Damage<ValTree>) {
        // "Order of Application"

        // 2. Resistance
        self.resistances
            .read()
            .iter()
            .for_each(|resistance| resistance.apply_to(damage));

        // 3. Vulnerability
        self.vulnerabilities
            .read()
            .iter()
            .for_each(|vulnerability| vulnerability.apply_to(damage));

        // (4. Immunity)
        self.immunities
            .read()
            .iter()
            .for_each(|immunity| immunity.apply_to_damage(damage));
    }
}

impl Default for RIV {
    fn default() -> Self {
        Self::new()
    }
}

pub trait DamageEffectTrait: xander_runtime::ui::Ui {
    const OP: d20::BinaryOperator;
    const RHS: u32;
}

fn has_label_recur<Op: DamageEffectTrait>(mut damage: &ValTree) -> bool {
    loop {
        println!("{damage:?}");
        let ValTree::Labeled(d20::Labeled(
            box ValTree::BinaryOperation(
                lhs,
                op,
                box ValTree::Literal(d20::Literal::Int(d20::Int(rhs))),
            ),
            label,
        )) = damage
        else {
            return false;
        };

        println!("{:?}", label.0);

        if label.0.as_ref().is_some_and(|l| l.is::<Op>()) && *op == Op::OP && *rhs == Op::RHS {
            return true;
        }

        damage = lhs;
    }
}

fn apply_op<Op, F>(filter: &DamageFilter, damage: &mut Damage<ValTree>, op: F)
where
    Op: DamageEffectTrait,
    F: for<'a> FnOnce(ValTree) -> ValTree,
{
    let Some(applicable) = (match filter {
        DamageFilter::Type(ty) => damage.get_mut(*ty),
        DamageFilter::Filter(single) => single.filter(damage),
    }) else {
        return;
    };

    // "They don't stack"
    if has_label_recur::<Op>(applicable) {
        return;
    }

    applicable.modify_in_place(op);
}

impl DamageEffectTrait for Resistance {
    const OP: d20::BinaryOperator = d20::BinaryOperator::IntDiv;
    const RHS: u32 = 2;
}

impl DamageEffectTrait for Vulnerability {
    const OP: d20::BinaryOperator = d20::BinaryOperator::Mul;
    const RHS: u32 = 2;
}

impl DamageEffectTrait for Immunity {
    const OP: d20::BinaryOperator = d20::BinaryOperator::Mul;
    const RHS: u32 = 0;
}

impl Resistance {
    pub fn apply_to(self: &Rc<Self>, damage: &mut Damage<ValTree>) {
        apply_op::<Self, _>(&self.0.to, damage, |lhs| {
            lhs.int_div(Self::RHS as i32).label(self.clone())
        });
    }
}

impl Vulnerability {
    pub fn apply_to(self: &Rc<Self>, damage: &mut Damage<ValTree>) {
        apply_op::<Self, _>(&self.0.to, damage, |lhs| {
            (lhs * Self::RHS as i32).label(self.clone())
        });
    }
}

impl Immunity {
    pub fn apply_to_damage(self: &Rc<Self>, damage: &mut Damage<ValTree>) {
        match &self.to {
            ImmunityTarget::Damage(filter) => {
                // This is fine -- I still want to keep the trail of operations on the damage.
                #[allow(clippy::erasing_op)]
                apply_op::<Immunity, _>(filter, damage, |lhs| {
                    (lhs * Self::RHS as i32).label(self.clone())
                });
            }
            ImmunityTarget::Condition(_) => todo!(),
        }
    }
}

pub mod ui {
    use xander_runtime::ui;

    use crate::engine::game::health::riv::{Immunity, Resistance, Vulnerability};

    impl ui::Ui for Resistance {}
    impl ui::Ui for Vulnerability {}
    impl ui::Ui for Immunity {}
}

register!(Resistance, register(Identity("HEALTH::RESISTANCE"), Lived(@)));
register!(
    Vulnerability,
    register(Identity("HEALTH::VULNERABILITY"), Lived(@))
);
register!(Immunity, register(Identity("HEALTH::IMMUNITY"), Lived(@)));
register!(
    DamageEffect,
    register(Identity("HEALTH::DAMAGE_EFFECT"), Lived(@))
);

#[cfg(test)]
mod tests {
    use std::ops::Mul;

    use super::*;

    #[test]
    fn test_recur_find() {
        let vulnerability = Rc::new(Vulnerability(DamageEffect {
            dep: OptionalDependency::new(None),
            to: DamageFilter::Type(DamageType::Acid),
        }));
        let resistance = Rc::new(Resistance(DamageEffect {
            dep: OptionalDependency::new(None),
            to: DamageFilter::Type(DamageType::Acid),
        }));

        // No effects.
        let damage = ValTree::from(2);
        assert!(!has_label_recur::<Resistance>(&damage));
        assert!(!has_label_recur::<Vulnerability>(&damage));

        // One effect.
        let damage = ValTree::from(13).int_div(2).label(resistance.clone());
        assert!(has_label_recur::<Resistance>(&damage));
        let damage = ValTree::from(13).mul(2).label(vulnerability.clone());
        assert!(has_label_recur::<Vulnerability>(&damage));

        // Both effects.
        let damage = ValTree::from(13)
            .int_div(2)
            .label(resistance.clone())
            .mul(2)
            .label(vulnerability.clone());

        assert!(has_label_recur::<Resistance>(&damage));
        assert!(has_label_recur::<Vulnerability>(&damage));

        let damage = ValTree::from(13)
            .mul(2)
            .label(vulnerability.clone())
            .int_div(2)
            .label(resistance.clone());

        assert!(has_label_recur::<Resistance>(&damage));
        assert!(has_label_recur::<Vulnerability>(&damage));
    }
}
