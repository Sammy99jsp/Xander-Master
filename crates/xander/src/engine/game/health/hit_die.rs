use crate::engine::game::creature::{CreatureSize, Me};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct HitDie {
    me: Me,
}

impl HitDie {
    pub fn new(me: Me) -> Self {
        Self { me }
    }
    
    pub fn die(&self) -> d20::DExpr {
        let me = &*self.me;
        match &me.kind {
            crate::engine::game::creature::CreatureKind::Character(_) => todo!(),
            crate::engine::game::creature::CreatureKind::Monster(_) => match me.size {
                CreatureSize::Tiny => d20::D4,
                CreatureSize::Small => d20::D6,
                CreatureSize::Medium => d20::D8,
                CreatureSize::Large => d20::D10,
                CreatureSize::Huge => d20::D12,
                CreatureSize::Gargantuan(_, _) => d20::D20,
            },
        }
    }
}

pub mod ui {
    use xander_runtime::ui;

    use crate::engine::game::health::hit_die::HitDie;

    impl ui::Ui for HitDie {}
}
