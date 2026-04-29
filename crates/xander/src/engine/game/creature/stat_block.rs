use rkyv::{
    de::Pooling,
    rancor::Source,
    ser::{Sharing, Writer},
    validation::{ArchiveContext, SharedContext},
};
use xander_runtime::lived::provided::Provided;

use crate::engine::game::{
    creature::{Me, proficiencies::Proficiencies},
    health::Health,
    stats::{Ability, AbilityModifier, AbilityScore, proficiency::ProficiencyBonus},
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(deserialize_bounds(__D: Pooling, __D::Error: Source), serialize_bounds(__S: Writer + Sharing, __S::Error: Source))]
#[rkyv(bytecheck(bounds(__C: ArchiveContext + SharedContext, __C::Error: Source)))]
pub struct StatBlock {
    #[rkyv(omit_bounds)]
    pub me: Me,
    pub proficiency_bonus: Provided<ProficiencyBonus>,
    pub proficiencies: Proficiencies,
    pub scores: AbilityScores,
    pub modifiers: AbilityModifiers,
    pub health: Health,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AbilityScores {
    pub str: Provided<AbilityScore>,
    pub dex: Provided<AbilityScore>,
    pub con: Provided<AbilityScore>,
    pub int: Provided<AbilityScore>,
    pub wis: Provided<AbilityScore>,
    pub cha: Provided<AbilityScore>,
}

/// Create a simple score with just the base ability score.
#[cfg(test)]
pub fn base_score(value: AbilityScore) -> Provided<AbilityScore> {
    let mut provided = Provided::new();
    provided.enroll_mut(provisos::BaseScore { value });
    provided
}

impl AbilityScores {
    pub async fn get(&self, ability: Ability) -> AbilityScore {
        let score = match ability {
            Ability::Strength => &self.str,
            Ability::Dexterity => &self.dex,
            Ability::Constitution => &self.con,
            Ability::Intelligence => &self.int,
            Ability::Wisdom => &self.wis,
            Ability::Charisma => &self.cha,
        };

        score.get().await
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AbilityModifiers {
    pub str: Provided<AbilityModifier>,
    pub dex: Provided<AbilityModifier>,
    pub con: Provided<AbilityModifier>,
    pub int: Provided<AbilityModifier>,
    pub wis: Provided<AbilityModifier>,
    pub cha: Provided<AbilityModifier>,
}

impl AbilityModifiers {
    pub fn new(stats: Me) -> Self {
        fn provided(stats: Me, ability: Ability) -> Provided<AbilityModifier> {
            let mut provided = Provided::new();
            provided.enroll_mut(provisos::BaseModifier { me: stats, ability });
            provided
        }

        Self {
            str: provided(stats.clone(), Ability::Strength),
            dex: provided(stats.clone(), Ability::Dexterity),
            con: provided(stats.clone(), Ability::Constitution),
            int: provided(stats.clone(), Ability::Intelligence),
            wis: provided(stats.clone(), Ability::Wisdom),
            cha: provided(stats, Ability::Charisma),
        }
    }

    pub async fn get(&self, ability: Ability) -> AbilityModifier {
        let modifier = match ability {
            Ability::Strength => &self.str,
            Ability::Dexterity => &self.dex,
            Ability::Constitution => &self.con,
            Ability::Intelligence => &self.int,
            Ability::Wisdom => &self.wis,
            Ability::Charisma => &self.cha,
        };

        modifier.get().await
    }
}

pub mod provisos {
    use crate::engine::game::{
        creature::Me,
        stats::{Ability, AbilityModifier, AbilityScore},
    };
    use xander_runtime::lived::provided::prelude::*;

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct BaseModifier {
        pub me: Me,
        pub ability: Ability,
    }

    identity!(BaseModifier: dyn ProvisoBase<AbilityModifier>, "MODIFIER_BASE");
    register!(BaseModifier: dyn ProvisoBase<AbilityModifier>, register(Archive, Deserialize, Lived));
    always_alive!(BaseModifier);

    impl ArchivedProvisoBase<AbilityModifier> for rkyv::Archived<BaseModifier> {}

    impl Proviso<AbilityModifier> for BaseModifier {
        const PRIORITY: usize = 0;

        fn provide(
            &self,
            t: &mut AbilityModifier,
        ) -> impl IntoFuture<Output = std::ops::ControlFlow<()>> {
            async move {
                *t = self.me.stats.scores.get(self.ability).await.modifier();

                std::ops::ControlFlow::Continue(())
            }
        }
    }

    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct BaseScore {
        pub value: AbilityScore,
    }

    impl std::fmt::Debug for BaseScore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("BaseScore")
                .field_with(|f| self.value.value().fmt(f))
                .finish()
        }
    }

    identity!(BaseScore: dyn ProvisoBase<AbilityScore>, "BASE_SCORE");
    register!(BaseScore: dyn ProvisoBase<AbilityScore>, register(Archive, Deserialize, Lived));
    always_alive!(BaseScore);

    impl ArchivedProvisoBase<AbilityScore> for rkyv::Archived<BaseScore> {}

    impl Proviso<AbilityScore> for BaseScore {
        const PRIORITY: usize = 0; // Always done first!
        fn provide(&self, t: &mut AbilityScore) -> impl IntoFuture<Output = ControlFlow<()>> {
            async {
                *t = self.value;
                ControlFlow::Continue(())
            }
        }
    }
}
