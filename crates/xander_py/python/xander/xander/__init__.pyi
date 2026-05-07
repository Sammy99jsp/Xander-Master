from . import templating as templating
from . import consts as consts

from typing import Generator, Generic, Literal as L, TypeVar
import numpy as np
from xander.pyutils import Direction

class Agent:
    def __init__(
        self,
        name: str,
        coroutine: Generator[None, Turn | Reaction, None],
        *,
        seed: L["random"] | int,
    ) -> None: ...

class Arena:
    def __init__(self, width: int, height: int) -> None: ...
    def random_square(self, *, seed: int | None = None) -> Position: ...
    def square_at(self, x: int, y: int) -> Position: ...

class Me:
    name: str
    hp: np.ndarray[tuple[L[2],], np.dtype[np.float32]]
    position: np.ndarray[tuple[L[2],], np.dtype[np.float32]]
    creature: Creature
    view: View
    len_actions: int

    def __repr__(self) -> str: ...
    def distance_from(self, other: Combatant) -> int: ...
    def displacement_from(
        self, other: Combatant
    ) -> np.ndarray[tuple[L[2],], np.dtype[np.float32]]: ...

class Creature: ...

class Game:
    def __init__(self, arena: Arena, debug: bool = False) -> None: ...
    def join(self, agent: Agent, creature: Creature, position: Position) -> Me: ...
    def start(self) -> None: ...

class Position: ...

class Movement:
    speed: int
    used: int
    left: int
    directions: np.ndarray[tuple[L[8]], np.dtype[np.bool]]

    def __repr__(self) -> str: ...

class Combatant:
    monster_type: str
    hp: np.ndarray[tuple[L[2],], np.dtype[np.float32]]
    position: np.ndarray[tuple[L[2],], np.dtype[np.float32]]
    initiative: int
    name: str

class View:
    me: Me
    allies: list[Combatant]
    enemies: list[Combatant]
    grid_me: np.ndarray[tuple[int, int], np.dtype[np.float32]]
    grid_allies: np.ndarray[tuple[int, int], np.dtype[np.float32]]
    grid_enemies: np.ndarray[tuple[int, int], np.dtype[np.float32]]
    arena_dims: np.ndarray[tuple[L[2],], np.dtype[np.float32]]

class Turn:
    movement: Movement
    actions: list[Availability[Action]]
    me: Me

    def end(self) -> None: ...
    def move(self, direction: Direction) -> Illegal | None: ...
    def take(self, action: Action) -> Illegal | AttackReport | None: ...
    def attack(self, attack: Attack) -> Illegal | AttackReport: ...
    def dash(self) -> Illegal | None: ...
    def disengage(self) -> Illegal | None: ...
    def dodge(self) -> Illegal | None: ...

class AttackOfOpportunity:
    actions: list[Availability[Attack]]
    target: Combatant
    me: Me

    def take(self, attack: Attack) -> AttackReport | Illegal: ...
    def skip(self) -> None: ...

class Reaction:
    __match_args__ = ("type",)
    type: AttackOfOpportunity
    me: Me
    actions: list[Availability[Action]]
    def take(self, attack: Attack) -> AttackReport | Illegal: ...

class Attack:
    name: str
    range: int | tuple[int, int]
    type: L["melee", "ranged"]
    damage: DamageDice
    target: Combatant

    def __repr__(self) -> str: ...

class AttackReport:
    def __repr__(self) -> str: ...
    damage: Damage | None
    to_hit: ValTree
    hit: bool

class Dash: ...
class Disengage: ...
class Dodge: ...

Action = Dash | Disengage | Dodge | Attack

class Illegal:
    def __init__(self, reason: str) -> None: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    reason: str

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
    me: Me
