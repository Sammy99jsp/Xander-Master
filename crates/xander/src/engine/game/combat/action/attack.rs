use std::rc::Rc;

use dynx::{Member, Namespace, dynx::Single};
use xander_runtime::{
    flow::{Event, dispatcher::DispatchState},
    futures::{FutureExt, future::LocalBoxFuture},
    register,
};

use crate::engine::{
    game::{
        Dispatcher,
        combat::{Combatant, Timeslot},
        creature::actions::AttackUseError,
        health::{
            Damage, DamageReport,
            damage::{DamageSource, DamageSourceType},
        },
        measure::Feet,
        stats::{
            Ability,
            d20_test::{
                D20Test,
                attack_roll::{
                    AttackRoll, AttackRollResult, Critical, events::PostAttackRollResultEvent,
                },
            },
            proficiency::ProficiencyApplicationBase,
        },
    },
    io::roller::DiceRollerError,
};

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Attack {
    pub name: String,
    pub base: Single<dyn AttackBase>,
    pub kind: AttackKind,
    pub hit: Damage<d20::DExpr>,
    pub prof: Option<Box<dyn ProficiencyApplicationBase>>,
    pub ability: Ability,
}

impl Attack {
    pub fn range(&self) -> Range {
        const DEFAULT_REACH: Feet = Feet(5);

        match &self.kind {
            AttackKind::Melee { reach, .. } => Range::Single(reach.unwrap_or(DEFAULT_REACH)),
            AttackKind::Ranged { range, .. } => *range,
        }
    }

    pub fn is_available(
        self: &Rc<Self>,
        slot: &Timeslot,
        me: &Rc<Combatant>,
        target: &Rc<Combatant>,
    ) -> Result<(), AttackUseError> {
        self.base.is_available(self, slot, me, target)
    }

    pub fn can_be_reaction(&self) -> bool {
        // TODO: Fix this over-simplification
        self.base.can_be_reaction(self)
    }

    pub(in crate::engine::game::combat) async fn attack(
        self: &Rc<Self>,
        slot: &Timeslot,
        me: &Rc<Combatant>,
        target: &Rc<Combatant>,
    ) -> Result<AttackReport, DiceRollerError> {
        self.base.execute(self, slot, me, target).await
    }

    pub async fn damage(self: &Rc<Self>, me: &Rc<Combatant>) -> Damage<d20::DExpr> {
        self.base.damage(self, me).await
    }

    pub fn attack_roll(&self, _me: &Rc<Combatant>, against: &Rc<Combatant>) -> AttackRoll {
        AttackRoll {
            against: Rc::downgrade(&against.creature),
            ability: self.ability,
            prof: self.prof.clone(),
        }
    }
}

register!(Attack, register(Identity("COMBAT::ATTACK"), Lived(@always)));

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum AttackKind {
    Melee { reach: Option<Feet> },
    Ranged { range: Range },
}

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum Range {
    Single(Feet),
    Long { short: Feet, long: Feet },
}

impl Range {
    pub fn within(&self, distance: Feet) -> bool {
        match self {
            Range::Single(range) => distance <= *range,
            Range::Long { short, long } => (*short <= distance) || (*long <= distance),
        }
    }
}

#[Namespace("ATTACK" @ NS, derive(Singleton))]
pub trait AttackBase: std::fmt::Debug {
    fn is_available(
        &self,
        attack: &Rc<Attack>,
        slot: &Timeslot,
        me: &Rc<Combatant>,
        target: &Rc<Combatant>,
    ) -> Result<(), AttackUseError>;
    fn can_be_reaction(&self, attack: &Attack) -> bool;
    fn damage<'a>(
        &'a self,
           attack: &'a Rc<Attack>,
        me: &'a Combatant,
    ) -> LocalBoxFuture<'a, Damage<d20::DExpr>>;
    fn execute<'a>(
        &'a self,
        attack: &'a Rc<Attack>,
        slot: &'a Timeslot,
        me: &'a Rc<Combatant>,
        target: &'a Rc<Combatant>,
    ) -> LocalBoxFuture<'a, Result<AttackReport, DiceRollerError>>;
}

#[derive(Debug)]
pub struct SetMonsterAttack;

#[Member("SET_MONSTER_ATTACK", register(Singleton))]
impl AttackBase for SetMonsterAttack {
    fn is_available(
        &self,
        attack: &Rc<Attack>,
        slot: &Timeslot,
        me: &Rc<Combatant>,
        target: &Rc<Combatant>,
    ) -> Result<(), AttackUseError> {
        if !attack.range().within(me.distance_between(target)) {
            return Err(AttackUseError::OutOfRange);
        }

        if matches!(slot, Timeslot::Reaction(_)) && matches!(&attack.kind, AttackKind::Ranged { .. }) {
            return Err(AttackUseError::OutOfTurn);
        }

        Ok(())
    }

    fn damage<'a>(
        &'a self,
        attack: &'a Rc<Attack>,
        me: &'a Combatant,
    ) -> LocalBoxFuture<'a, Damage<d20::DExpr>> {
        async {
            let hit = attack
                .hit
                .clone()
                .map(|_, d| d.label(Rc::new(ui::OriginalDamageDice)));
            let modifier: d20::DExpr = me.creature.stats.modifiers.get(attack.ability).await.into();
            
            if hit.types() > 1 {
                todo!("Don't really know what to do in this case...\n*which* damage type is the ability modifier added to?");
            }
            
            hit.map(|_, d| d + modifier.clone().label(Rc::new(ui::AbilityModifier { me: Rc::downgrade(&me.creature), attack: Rc::downgrade(attack) })))
        }
        .boxed_local()
    }

    fn can_be_reaction(&self, attack: &Attack) -> bool {
        match &attack.kind {
            AttackKind::Melee { .. } => true,
            AttackKind::Ranged { .. } => false,
        }
    }

    fn execute<'a>(
        &'a self,
        attack: &'a Rc<Attack>,
        _: &'a Timeslot,
        me: &'a Rc<Combatant>,
        target: &'a Rc<Combatant>,
    ) -> LocalBoxFuture<'a, Result<AttackReport, DiceRollerError>> {
        async move {
            let game = Dispatcher::local().await;

            // Calculate the appropriate damage.
            let damage = self.damage(attack, me).await;

            // Ranged attack checks.
            if let AttackKind::Ranged { range } = &attack.kind {
                const CLOSE_COMBAT: Feet = Feet(5);
                let distance = me.distance_between(target);

                // Ranged Attacks in Close Combat
                // TODO: Check for incapacitated condition.
                if distance <= CLOSE_COMBAT {
                    game.listen(handlers::RangedAttackDisadvantage::new(
                        Rc::downgrade(&me.creature),
                        ui::RangedAttackCloseCombatHint {
                            distance,
                            me: Rc::downgrade(&me.creature),
                            target: Rc::downgrade(&target.creature),
                            attack: Rc::downgrade(attack),
                        },
                    ));
                }

                match range {
                    // "Your attack roll has Disadvantage when your
                    // target is beyond normal range [...]"
                    Range::Long {
                        short: normal,
                        long,
                    } if distance > *normal && distance <= *long => {
                        game.listen(handlers::RangedAttackDisadvantage::new(
                            Rc::downgrade(&me.creature),
                            ui::RangedAttackFarDistanceHint {
                                distance,
                                me: Rc::downgrade(&me.creature),
                                target: Rc::downgrade(&target.creature),
                                attack: Rc::downgrade(attack),
                            },
                        ));
                    }
                    _ => (),
                }
            }

            // Perform the attack roll...
            let PostAttackRollResultEvent {
                attack_roll,
                attack_roll_result,
                result,
                attacker,
            } = attack
                .attack_roll(me, target)
                .perform(me)
                .await
                .into_result()
                .into_ok();

            // If we have missed, skip damage rolling and just return...
            let critical = match result {
                AttackRollResult::Miss => {
                    return Ok(AttackReport {
                        attack_roll,
                        attack_roll_result,
                        result: AttackResult::Miss,
                    });
                }
                AttackRollResult::Hit(critical) => critical,
            };

            // This is for certain abilities that allow you to
            // apply extra damage after a hit.
            let events::PreAttackDamageRollEvent {
                attack,
                attack_roll_result,
                critical,
                mut damage,
                ..
            } = events::PreAttackDamageRollEvent {
                attacker,
                attack: attack.clone(),
                critical,
                damage,
                attack_roll_result,
            }
            .handle()
            .await
            .into_result()
            .into_ok();

            // "When you score a Critical Hit, you deal extra dam-
            // age. Roll the attack’s damage dice twice, add them
            // together [...]"
            if critical.is_some() {
                damage
                    .as_mut()
                    .filter_map(|_, expr| expr.find_labelled_mut::<ui::OriginalDamageDice>())
                    .for_each(|_, damage| {
                        damage.traverse_mut(|expr| {
                            if let d20::DExpr::Dice(dice) = expr {
                                dice.qty = Some(d20::Int(dice.qty() * 2));
                            }

                            expr.modify_in_place(|expr| {
                                expr.label(Rc::new(ui::CriticalHit {
                                    me: Rc::downgrade(&me.creature),
                                    target: Rc::downgrade(&target.creature),
                                    attack: Rc::downgrade(&attack),
                                }))
                            });
                        })
                    });
            }

            // Roll for damage.
            let agent = me.actor.state(&game.interface);
            let damage = damage.roll(agent.roller()).await?;

            let events::PostAttackDamageRollEvent {
                attack,
                attack_roll_result,
                critical,
                damage,
            } = events::PostAttackDamageRollEvent {
                attack,
                critical,
                damage,
                attack_roll_result,
            }
            .handle()
            .await
            .into_result()
            .into_ok();

            // Deal the damage.
            let report = target
                .creature
                .stats
                .health
                .damage(
                    damage,
                    DamageSource {
                        from: Some(Rc::downgrade(me)),
                        ty: DamageSourceType::Attack(Rc::downgrade(&attack)),
                    },
                )
                .await
                .ok();

            Ok(AttackReport {
                attack_roll,
                attack_roll_result,
                result: AttackResult::Hit { critical, report },
            })
        }
        .boxed_local()
    }
}

#[derive(Debug)]
pub enum AttackResult {
    Miss,
    Hit {
        critical: Option<Critical>,
        report: Option<DamageReport>,
    },
}

#[derive(Debug)]
pub struct AttackReport {
    pub attack_roll: AttackRoll,
    pub attack_roll_result: d20::ValTree,
    pub result: AttackResult,
}

pub mod events {
    use std::{
        future::ready,
        rc::{Rc, Weak},
    };

    use xander_runtime::{
        flow::{Event, event::EventBase},
        register,
    };

    use crate::engine::game::{
        Game, combat::Attack, creature::Creature, health::Damage,
        stats::d20_test::attack_roll::Critical,
    };

    #[derive(Debug)]
    pub struct PreAttackDamageRollEvent {
        pub attacker: Weak<Creature>,
        pub attack: Rc<Attack>,
        pub attack_roll_result: d20::ValTree,
        pub critical: Option<Critical>,
        pub damage: Damage<d20::DExpr>,
    }

    register!(PreAttackDamageRollEvent: dyn EventBase<Game>, register(Identity("ATTACK::PRE_DAMAGE_ROLL")));

    impl Event<Game> for PreAttackDamageRollEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { unreachable!() }
        }
    }

    #[derive(Debug)]
    pub struct PostAttackDamageRollEvent {
        pub attack: Rc<Attack>,
        pub attack_roll_result: d20::ValTree,
        pub critical: Option<Critical>,
        pub damage: Damage<d20::ValTree>,
    }

    register!(PostAttackDamageRollEvent: dyn EventBase<Game>, register(Identity("ATTACK::POST_DAMAGE_ROLL")));

    impl Event<Game> for PostAttackDamageRollEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { unreachable!() }
        }
    }
}

pub mod ui {
    use std::rc::Weak;

    use xander_runtime::ui::Ui;

    use crate::engine::game::{combat::Attack, creature::Creature, measure::Feet};

    #[derive(Debug)]
    pub struct OriginalDamageDice;

    impl Ui for OriginalDamageDice {}
    
    #[derive(Debug)]
    pub struct AbilityModifier {
        pub me: Weak<Creature>,
        pub attack: Weak<Attack>,
    }

    impl Ui for AbilityModifier {}

    #[derive(Debug)]
    pub struct CriticalHit {
        pub me: Weak<Creature>,
        pub target: Weak<Creature>,
        pub attack: Weak<Attack>,
    }

    impl Ui for CriticalHit {}

    #[derive(Debug)]
    pub struct RangedAttackCloseCombatHint {
        pub distance: Feet,
        pub me: Weak<Creature>,
        pub target: Weak<Creature>,
        pub attack: Weak<Attack>,
    }

    impl Ui for RangedAttackCloseCombatHint {}

    #[derive(Debug)]
    pub struct RangedAttackFarDistanceHint {
        pub distance: Feet,
        pub me: Weak<Creature>,
        pub target: Weak<Creature>,
        pub attack: Weak<Attack>,
    }

    impl Ui for RangedAttackFarDistanceHint {}
}

pub mod handlers {
    use std::{
        cell::{Cell, RefCell},
        rc::{Rc, Weak},
    };

    use xander_runtime::{Lived, flow::event::EventHandler, ui::Ui};

    use crate::engine::game::{
        Game,
        creature::Creature,
        stats::d20_test::{D20TestRoll, Disadvantage, attack_roll::events::PreAttackRollEvent},
    };

    #[derive(Debug)]
    pub struct RangedAttackDisadvantage<Hint> {
        me: Weak<Creature>,
        applied: Cell<bool>,
        hint: RefCell<Option<Hint>>,
    }

    impl<Hint> RangedAttackDisadvantage<Hint> {
        pub fn new(me: Weak<Creature>, hint: Hint) -> Self {
            Self {
                me,
                applied: Cell::default(),
                hint: RefCell::new(Some(hint)),
            }
        }
    }

    impl<Hint> Lived for RangedAttackDisadvantage<Hint> {
        fn is_alive(&self) -> bool {
            !self.applied.get()
        }
    }

    impl<Hint> EventHandler<Game> for RangedAttackDisadvantage<Hint>
    where
        Hint: Ui,
    {
        type Event = PreAttackRollEvent;

        fn handle<'s, 'e: 's>(
            &'s self,
            event: &'e mut Self::Event,
        ) -> impl IntoFuture<Output = ()> + 's {
            async {
                if !event.attacker.ptr_eq(&self.me) {
                    return;
                }

                event.test_dice = event.test_dice.impose(Disadvantage {
                    reason: Some(Rc::new(self.hint.borrow_mut().take().unwrap())),
                });

                self.applied.set(true);
            }
        }
    }
}

pub fn test_attack(name: &str) -> Attack {
    use crate::engine::game::health::DamageType;
    Attack {
        name: name.to_string(),
        base: Single::new(&SetMonsterAttack),
        kind: AttackKind::Melee { reach: None },
        hit: Damage::of(DamageType::Bludgeoning, d20::D4),
        prof: None,
        ability: Ability::Strength,
    }
}
