from typing import Generator, Literal as L, TypeAlias

from xander import Attack
from xander.xander import Dash, Disengage, Dodge, Turn, Reaction, GameEnd


Direction: TypeAlias = L[0, 1, 2, 3, 4, 5, 6, 7]
Event: TypeAlias = Turn | Reaction | GameEnd
CombatantCoroutine: TypeAlias = Generator[None, Event, None]
Action: TypeAlias = Dash | Disengage | Dodge | Attack


class Directions:
    UP = 0
    TOP_RIGHT = 1
    RIGHT = 2
    BOTTOM_RIGHT = 3
    BOTTOM = 4
    BOTTOM_LEFT = 5
    LEFT = 6
    TOP_LEFT = 7
