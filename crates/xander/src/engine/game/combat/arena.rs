use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use xander_runtime::{DynWeak, dynx::cells::InnerValue, lived::LivedList};

use crate::engine::game::{combat::Combatant, magic::aoe::AreaOfEffect};

#[derive(Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Square {
    #[rkyv(with = InnerValue<Vec<Weak<Combatant>>>)]
    pub occupants: RefCell<Vec<Weak<Combatant>>>,
    pub effects: LivedList<DynWeak<dyn AreaOfEffect>>,
}

impl Square {
    pub const fn new() -> Self {
        Self {
            effects: LivedList::new(),
            occupants: RefCell::new(Vec::new()),
        }
    }

    pub fn is_occupied(&self) -> bool {
        // TODO: Actually implement the rules regarding tiny sizes, unconsciousness, etc.
        self.occupants
            .borrow()
            .iter()
            .any(|a| !a.upgrade().unwrap().creature.is_dead())
    }

    pub fn remove_occupant(&self, me: &Rc<Combatant>) {
        let mut occupants = self.occupants.borrow_mut();
        let index = occupants
            .iter()
            .position(|a| std::ptr::addr_eq(a.as_ptr(), Rc::as_ptr(me)));

        if let Some(index) = index {
            occupants.swap_remove(index);
        }
    }

    pub fn add_occupant(&self, me: &Rc<Combatant>) {
        let mut occupants = self.occupants.borrow_mut();
        occupants.push(Rc::downgrade(me));
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn check_pos(&self, pos: Position) -> Option<Position> {
        (pos.x < self.width && pos.y < self.height).then_some(pos)
    }
}

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

pub const DIRECTIONS: [[i32; 2]; 8] = [
    [0, -1],  // Up
    [1, -1],  // Top-Right
    [1, 0],   // Right
    [1, 1],   // Bottom-Right
    [0, 1],   // Down
    [-1, 1],  // Bottom-Left
    [-1, 0],  // Left
    [-1, -1], // Top-Left
];

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Arena {
    grid: Box<[Square]>,
    dimensions: Dimensions,
}

impl Arena {
    pub fn new(dimensions: Dimensions) -> Self {
        let mut grid = Vec::new();
        let n = dimensions.width as usize * dimensions.height as usize;
        grid.reserve_exact(n);

        for _ in 0..n {
            grid.push(Square::new());
        }

        Self {
            grid: grid.into_boxed_slice(),
            dimensions,
        }
    }

    #[inline]
    fn flat_index(&self, pos: Position) -> Option<usize> {
        self.dimensions
            .check_pos(pos)
            .map(|Position { x, y }| (y * self.dimensions.width + x) as usize)
    }

    pub fn at(&self, pos: Position) -> Option<&Square> {
        self.grid.get(self.flat_index(pos)?)
    }

    pub fn distance(from: Position, to: Position) -> u32 {
        const fn sgn(x: i32) -> i32 {
            if x < 0 { -1 } else { 1 }
        }

        let (delta_x, delta_y) = (to.x as i32 - from.x as i32, to.y as i32 - from.y as i32);
        let c = i32::min(delta_x, delta_y);
        let a = sgn(delta_x) * (delta_x - sgn(delta_x) * c);
        let b = sgn(delta_y) * (delta_y - sgn(delta_y) * c);

        let distance = a + b + c;
        debug_assert!(distance >= 0);

        distance as u32
    }

    pub fn around(&self, pos @ Position { x, y }: Position) -> Option<Around<'_>> {
        self.flat_index(pos).map(|_| Around {
            clockwise: DIRECTIONS.map(|[dx, dy]| {
                x.checked_add_signed(dx)
                    .zip(y.checked_add_signed(dy))
                    .and_then(|(x, y)| self.at(Position { x, y }))
            }),
        })
    }
}

/// Shows the squares around a point
#[derive(Debug)]
pub struct Around<'a> {
    pub clockwise: [Option<&'a Square>; 8],
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::Position;
    use crate::engine::game::{
        combat::{
            Combatant,
            arena::{Arena, DIRECTIONS},
        },
        creature::test_creature,
    };

    #[test]
    fn test_thingy() {
        let arena = Arena::new(super::Dimensions {
            width: 5,
            height: 5,
        });

        let creature = test_creature();
        let combatant = Rc::new(Combatant {
            creature,
            initiative_score: 20,
            position: Cell::new(Position { x: 0, y: 0 }),
        });

        arena
            .at(Position { x: 0, y: 0 })
            .unwrap()
            .occupants
            .borrow_mut()
            .push(Rc::downgrade(&combatant));

        let around = arena.around(Position { x: 0, y: 5343 });

        if let Some(around) = around {
            around
                .clockwise
                .iter()
                .enumerate()
                .for_each(|(i, sq)| println!("{:?} => {sq:?}", DIRECTIONS[i]));
        }
    }
}
