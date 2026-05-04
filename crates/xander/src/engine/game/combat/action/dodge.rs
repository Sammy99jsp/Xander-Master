use std::{
    future::ready,
    rc::{Rc, Weak},
};

use dynx::Member;
use xander_runtime::{Lived, dependently_alive, flow::event::EventHandler};

use crate::engine::game::{
    Game,
    combat::utils::NextTurn,
    creature::{
        Creature,
        marker::{ArchivedMarker, Marker},
    },
    stats::{
        Ability,
        d20_test::{Advantage, D20TestRoll, Disadvantage, attack_roll, save},
    },
};

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dodging {
    pub me: Weak<Creature>,
    pub next_turn: NextTurn,
}

impl Dodging {
    pub async fn apply(self) -> Rc<Self> {
        let rc = Rc::new(self);
        let weak = Rc::downgrade(&rc);
        DodgeAttackRolls(weak.clone()).listen().await;
        DodgeDexSaves(weak).listen().await;
        rc
    }
}

impl Dodging {
    pub fn ui_hint(&self) -> ui::DodgeUiHint {
        ui::DodgeUiHint {
            me: self.me.clone(),
        }
    }

    fn has_lost_benefit(&self) -> bool {
        // TODO: Check if incapacitated or speed == 0.
        false
    }
}

impl Lived for Dodging {
    fn is_alive(&self) -> bool {
        !(self.next_turn.yet() || self.has_lost_benefit())
    }
}

#[Member("ACTION::DODGING", register(Archive, Deserialize))]
impl Marker for Dodging {}
impl ArchivedMarker for rkyv::Archived<Dodging> {}

/// > Until the start of your next turn, attack
/// > rolls against you have Disadvantage
#[derive(Debug)]
struct DodgeAttackRolls(pub Weak<Dodging>);
dependently_alive!(DodgeAttackRolls, 0);

impl EventHandler<Game> for DodgeAttackRolls {
    type Event = attack_roll::events::PreAttackRollEvent;

    fn handle<'s, 'e: 's>(
        &'s self,
        event: &'e mut Self::Event,
    ) -> impl IntoFuture<Output = ()> + 's {
        let dodging: Rc<Dodging> = self.0.upgrade().unwrap();
        if !event.attack_roll.against.ptr_eq(&dodging.me) {
            return ready(());
        }

        event.test_dice = event.test_dice.impose(Disadvantage {
            reason: Some(Rc::new(dodging.ui_hint())),
        });

        ready(())
    }
}

/// > Until the start of your next turn, \[...\]
/// > you make Dexterity saving throws with Advantage.
#[derive(Debug)]
struct DodgeDexSaves(pub Weak<Dodging>);
dependently_alive!(DodgeDexSaves, 0);

impl EventHandler<Game> for DodgeDexSaves {
    type Event = save::events::PreRollSaveEvent;

    fn handle<'s, 'e: 's>(
        &'s self,
        event: &'e mut Self::Event,
    ) -> impl IntoFuture<Output = ()> + 's {
        let dodging: Rc<Dodging> = self.0.upgrade().unwrap();

        if !event.creature.ptr_eq(&dodging.me) || event.save.ability != Ability::Dexterity {
            return ready(());
        }

        event.test_dice = event.test_dice.grant(Advantage {
            reason: Some(Rc::new(dodging.ui_hint())),
        });

        ready(())
    }
}

pub mod ui {
    use std::rc::Weak;

    use xander_runtime::ui::Ui;

    use crate::engine::game::creature::Creature;

    #[derive(Debug)]
    pub struct DodgeUiHint {
        pub me: Weak<Creature>,
    }

    impl Ui for DodgeUiHint {}
}
