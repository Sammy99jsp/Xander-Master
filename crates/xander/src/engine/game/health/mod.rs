pub mod current_hp;
pub mod damage;
pub mod death;
pub mod hit_die;
pub mod max_hp;
pub mod riv;
pub mod temp_hp;

use std::{cell::Cell, num::NonZeroU32};

use xander_runtime::{dynx::cells::InnerValue, flow::Event, lived::cell::LivedCell};

pub(super) type HpValue = u32;

use crate::engine::game::{
    creature::{CreatureKind, Me},
    health::{damage::DamageSource, death::Dead, riv::RIV, temp_hp::Discounted},
};

pub use self::{
    current_hp::CurrentHp,
    damage::{Damage, DamageType},
    hit_die::HitDie,
    max_hp::MaxHp,
    temp_hp::TempHp,
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Health {
    me: Me,
    pub hit_die: HitDie,
    pub temp_hp: LivedCell<TempHp>,
    pub max_hp: MaxHp,
    current: CurrentHp,
    pub riv: RIV,
    #[rkyv(with = InnerValue<Option<Dead>>)]
    dead: Cell<Option<Dead>>,
}

impl Health {
    pub fn new(me: Me) -> Self {
        Self {
            me: me.clone(),
            hit_die: HitDie::new(me),
            temp_hp: LivedCell::empty(),
            max_hp: MaxHp::new(),
            current: CurrentHp::new(0),
            riv: RIV::new(),
            dead: Cell::default(),
        }
    }

    pub fn with_set_max(me: Me, max_hp: u32) -> Option<Self> {
        Some(Self {
            me: me.clone(),
            hit_die: HitDie::new(me),
            temp_hp: LivedCell::empty(),
            max_hp: MaxHp::with_set(max_hp)?,
            current: CurrentHp::new(max_hp),
            riv: RIV::new(),
            dead: Default::default(),
        })
    }

    pub fn current(&self) -> HpValue {
        self.current.value.get()
    }

    pub async fn damage(
        &self,
        damage: Damage<d20::ValTree>,
        source: DamageSource,
    ) -> Result<DamageReport, events::CreatureHurtCancelledReason> {
        // "Order of Application"
        // 1. Circumstantial Adjustments
        let events::CreatureHurtEvent { mut damage, .. } = events::CreatureHurtEvent {
            cancelled: Default::default(),
            damage,
            source,
        }
        .handle()
        .await
        .into_result()?;

        // Apply 2. and 3. (Resistance, Vulnerability, then Immunity)
        self.riv.apply_to_damage(&mut damage);

        // TODO: Fire Pre-damage taken events.

        let mut report = {
            let total = match damage.total() {
                ..0 => {
                    return Ok(DamageReport {
                        discounted: None,
                        total: 0,
                        dealt: damage,
                        outcome: DamageOutcome::Nothing,
                    });
                }
                pos => pos as u32,
            };

            DamageReport {
                discounted: None,
                total,
                dealt: damage,
                outcome: DamageOutcome::Hurt,
            }
        };

        // "Lose Temporary Hit Points First"
        if let Some(temp_hp) = self.temp_hp.get().as_ref() {
            let Discounted {
                discounted,
                remaining,
            } = temp_hp.discount(report.total);

            report.total = remaining;
            report.discounted = Some(discounted);
        }

        // Now actually take the damage.
        let (hp_after, excess) = {
            let current_hp = self.current.value.get();

            let hp_after = current_hp.saturating_sub(report.total);
            let excess = report.total.saturating_sub(current_hp);

            self.current.value.set(hp_after);

            (hp_after, excess)
        };

        let creature_kind = &self.me.kind;

        match (creature_kind, hp_after, excess) {
            // Take damage as normal.
            (_, 1.., 0) => (),
            (_, 1.., 1..) => unreachable!("Should not have excess HP!"),

            // Instant Death -- Monster Death
            (CreatureKind::Monster(_), 0, _) => {
                // TODO: Allow the exception at the GM's choice, using [Decision].
                self.dead.set(Some(Dead));
                report.outcome = DamageOutcome::Killed;
            }

            // Instant Death -- Massive Damage
            (_, 0, xs) if xs >= self.max_hp.get().await => {
                self.dead.set(Some(Dead));
                report.outcome = DamageOutcome::Killed;
            }

            // Falling Unconscious
            (CreatureKind::Character(_), 0, _) => todo!(),
        }

        // TODO: Fire post-damage taken event.
        Ok(report)
    }

    pub fn is_dead(&self) -> bool {
        self.dead.get().is_some()
    }
}

#[derive(Debug)]
pub struct DamageReport {
    pub dealt: Damage<d20::ValTree>,
    pub total: u32,
    pub discounted: Option<NonZeroU32>,
    pub outcome: DamageOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
    Nothing,
    Hurt,
    KnockedOut,
    Killed,
}

pub mod events {
    use std::{future::ready, rc::Weak};

    use xander_runtime::{
        flow::{Event, event::EventBase},
        register, ui,
    };

    use crate::engine::game::{
        Game,
        health::{Damage, damage::DamageSource},
    };

    #[derive(Debug)]
    pub struct CreatureHurtCancelledReason {
        pub reason: Weak<dyn ui::Ui>,
    }

    #[derive(Debug)]
    pub struct CreatureHurtEvent {
        pub cancelled: Option<CreatureHurtCancelledReason>,
        pub damage: Damage<d20::ValTree>,
        pub source: DamageSource,
    }

    register!(CreatureHurtEvent: dyn EventBase<Game>, register(Identity("HEALTH::CREATURE_HURT")));

    impl EventBase<Game> for CreatureHurtEvent {
        fn is_cancelled(&self) -> bool {
            self.cancelled.is_some()
        }
    }

    impl Event<Game> for CreatureHurtEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = CreatureHurtCancelledReason;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            ready(self.cancelled.unwrap())
        }
    }
}

pub mod decisions {}

pub mod rulings {}
