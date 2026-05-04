from io import TextIOWrapper
from typing import Generator, Generic, Literal as L, TypeVar
import numpy as np
from xander.pyutils import Direction

class Agent:
    def __init__(
        self,
        name: str,
        coroutine: Generator[None, Turn | Reaction, None],
        *,
        seed: int | None = None,
    ) -> None: ...

class Arena:
    def __init__(self, width: int, height: int) -> None: ...
    def random_square(self) -> Position: ...
    def square_at(self, x: int, y: int) -> Position: ...

class Combatant:
    name: str
    current_hp: int
    max_hp: int
    creature: Creature

    def __repr__(self) -> str: ...

class Creature: ...

class Game:
    def __init__(self, arena: Arena) -> None: ...
    def load_creature_json(
        self, path: str | TextIOWrapper, *, name: str | None = None
    ) -> Creature: ...
    def join(
        self, agent: Agent, creature: Creature, position: Position
    ) -> Combatant: ...
    def start(self) -> None: ...

class Position: ...

class Movement:
    speed: int
    used: int
    left: int
    directions: np.ndarray[tuple[L[8]], np.dtype[np.float32]]

    def __repr__(self) -> str: ...

class Turn:
    movement: Movement
    actions: list[Availability[Action]]

    def end(self) -> None: ...
    def move(self, direction: Direction) -> Illegal | None: ...
    def take(self, action: Action) -> Illegal | AttackReport | None: ...
    def attack(self, attack: Attack) -> Illegal | AttackReport: ...
    def dash(self) -> Illegal | None: ...
    def disengage(self) -> Illegal | None: ...
    def dodge(self) -> Illegal | None: ...

class AttackOfOpportunity:
    attacks: list[Availability[Attack]]
    target: Combatant

    def attack(self, attack: Attack) -> AttackReport | Illegal: ...
    def skip(self) -> None: ...

class Reaction:
    __match_args__ = ("type",)
    type: AttackOfOpportunity

class Attack:
    name: str
    range: int | tuple[int, int]
    type: L["melee", "ranged"]
    damage: DamageDice

    def __repr__(self) -> str: ...

class AttackReport:
    def __repr__(self) -> str: ...
    damage: Damage | None
    hit: bool

class Dash: ...
class Disengage: ...
class Dodge: ...

Action = Dash | Disengage | Dodge | Attack

class Illegal:
    def __repr__(self) -> str: ...

class DExpr:
    def __repr__(self) -> str: ...

class ValTree:
    def __repr__(self) -> str: ...
    def __int__(self) -> int: ...
    def total(self) -> int: ...

class DamageDice:
    def __repr__(self) -> str: ...
    def sum(self) -> DExpr: ...

class Damage:
    def __repr__(self) -> str: ...
    def __int__(self) -> int: ...
    def sum(self) -> ValTree: ...
    def total(self) -> int: ...

class DamageType:
    Acid: DamageType
    Bludgeoning: DamageType
    Cold: DamageType
    Fire: DamageType
    Force: DamageType
    Lighting: DamageType
    Necrotic: DamageType
    Piercing: DamageType
    Poison: DamageType
    Psychic: DamageType
    Radiant: DamageType
    Slashing: DamageType
    Thunder: DamageType

T = TypeVar("T")

class Availability(Generic[T]):
    def is_available(self: Availability[T]) -> bool: ...
    def value(self: Availability[T]) -> T: ...
    def __repr__(self) -> str: ...

class GameEnd:
    won: bool