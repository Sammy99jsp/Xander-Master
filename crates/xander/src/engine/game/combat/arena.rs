use std::{cell::RefCell, rc::Weak};

use xander_runtime::{DynWeak, dynx::cells::InnerValue, lived::LivedList};

use crate::engine::game::{combat::Combatant, magic::aoe::AreaOfEffect, measure::Squares};

#[derive(Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Square {
    #[rkyv(with = InnerValue<Vec<Weak<Combatant>>>)]
    occupants: RefCell<Vec<Weak<Combatant>>>,
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

    pub fn remove_occupant(&self, me: Weak<Combatant>) {
        let mut occupants = self.occupants.borrow_mut();
        let index = occupants.iter().position(|a| a.ptr_eq(&me));

        if let Some(index) = index {
            occupants.swap_remove(index);
        }
    }

    pub fn add_occupant(&self, me: Weak<Combatant>) {
        let mut occupants = self.occupants.borrow_mut();
        occupants.push(me);
    }

    pub fn occupants(&self) -> Vec<Weak<Combatant>> {
        self.occupants.borrow().clone()
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dimensions {
    pub width: Squares,
    pub height: Squares,
}

impl Dimensions {
    pub fn check_pos(&self, pos: Position) -> Option<Position> {
        (pos.x < self.width && pos.y < self.height).then_some(pos)
    }
}

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Position {
    pub x: Squares,
    pub y: Squares,
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
        let n = dimensions.width.0 as usize * dimensions.height.0 as usize;
        grid.reserve_exact(n);

        for _ in 0..n {
            grid.push(Square::new());
        }

        Self {
            grid: grid.into_boxed_slice(),
            dimensions,
        }
    }

    #[cfg(test)]
    pub fn test() -> Self {
        Self::new(Dimensions {
            width: Squares(5),
            height: Squares(5),
        })
    }

    #[inline]
    fn flat_index(&self, pos: Position) -> Option<usize> {
        self.dimensions
            .check_pos(pos)
            .map(|Position { x, y }| (y.0 * self.dimensions.width.0 + x.0) as usize)
    }

    pub fn at(&self, pos: Position) -> Option<&Square> {
        self.grid.get(self.flat_index(pos)?)
    }

    #[inline]
    pub fn distance(from: Position, to: Position) -> Squares {
        #[inline]
        const fn sgn(x: i32) -> i32 {
            if x < 0 { -1 } else { 1 }
        }

        let (delta_x, delta_y) = (
            to.x.0 as i32 - from.x.0 as i32,
            to.y.0 as i32 - from.y.0 as i32,
        );
        let c = i32::min(delta_x.abs(), delta_y.abs());
        let a = sgn(delta_x) * (delta_x - sgn(delta_x) * c);
        let b = sgn(delta_y) * (delta_y - sgn(delta_y) * c);

        let distance = a + b + c;
        debug_assert!(distance >= 0);

        Squares(distance as u32)
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

    pub fn display_debug(&self) -> String {
        let displ_w = self.dimensions.width.0 as usize * 2 + 1 + 2;
        let displ_h = self.dimensions.height.0 as usize * 2 + 2;

        const CORNER: [[char; 2]; 2] = [['┌', '┐'], ['└', '┘']];
        const LINES: [char; 2] = ['─', '│'];
        const EMPTY: char = '·';
        const NEW_LINE: char = '\n';

        let mut output = String::with_capacity(displ_h * (displ_w + 1));

        {
            output.push(CORNER[0][0]);
            (0..(displ_w - 2)).for_each(|_| output.push(LINES[0]));
            output.push(CORNER[0][1]);
            output.push(NEW_LINE);
        }

        for row in self.grid.chunks(self.dimensions.width.0 as usize) {
            output.push(LINES[1]);
            output.push(' ');
            for sq in row {
                #[rustfmt::skip]
                let o = sq.occupants.borrow()
                .first().cloned()
                .as_ref().and_then(Weak::upgrade)
                .and_then(|a|a.creature.name.chars().next())
                .unwrap_or(EMPTY);

                output.push(o);
                output.push(' ');
            }
            output.push(LINES[1]);
            output.push(NEW_LINE);
        }

        {
            output.push(CORNER[1][0]);
            (0..(displ_w - 2)).for_each(|_| output.push(LINES[0]));
            output.push(CORNER[1][1]);
            output.push(NEW_LINE);
        }

        output
    }
}

/// Shows the squares around a point
#[derive(Debug)]
pub struct Around<'a> {
    pub clockwise: [Option<&'a Square>; 8],
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Position;
    use crate::engine::game::{
        combat::arena::{Arena, DIRECTIONS},
        creature::test_combatant,
        measure::Squares,
    };

    #[test]
    fn around() {
        let arena = Arena::new(super::Dimensions {
            width: Squares(5),
            height: Squares(5),
        });

        let combatant = test_combatant();

        arena
            .at(Position {
                x: Squares(0),
                y: Squares(0),
            })
            .unwrap()
            .occupants
            .borrow_mut()
            .push(Rc::downgrade(&combatant));

        let around = arena.around(Position {
            x: Squares(0),
            y: Squares(5343),
        });

        if let Some(around) = around {
            around
                .clockwise
                .iter()
                .enumerate()
                .for_each(|(i, sq)| println!("{:?} => {sq:?}", DIRECTIONS[i]));
        }
    }

    #[test]
    pub fn distance_between() {
        fn dist(p1: (u32, u32), p2: (u32, u32)) -> u32 {
            let from = Position {
                x: Squares(p1.0),
                y: Squares(p1.1),
            };
            let to = Position {
                x: Squares(p2.0),
                y: Squares(p2.1),
            };
            Arena::distance(from, to).0
        }

        let d1 = dist((3, 3), (2, 2));
        assert_eq!(d1, 1);
    }
}
