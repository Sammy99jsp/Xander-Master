from typing import Generator, Literal as L

from xander import Attack
from xander.xander import Dash, Disengage, Dodge, Turn, Reaction, GameEnd


Direction = L[0, 1, 2, 3, 4, 5, 6, 7]
AgentCoroutine = Generator[None, Turn | Reaction | GameEnd, None]
Action = Dash | Disengage | Dodge | Attack


class DIRECTIONS:
    UP = 0
    TOP_RIGHT = 1
    RIGHT = 2
    BOTTOM_RIGHT = 3
    BOTTOM = 4
    BOTTOM_LEFT = 5
    LEFT = 6
    TOP_LEFT = 7
