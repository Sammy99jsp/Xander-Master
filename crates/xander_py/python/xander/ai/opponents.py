"""
Pre-built, algorithmic opponents that can be used during training.
"""

from typing import Any, Callable, Generator, Literal, TypedDict
import random as rand

import numpy as np

from .. import xander as X
from ..pyutils import CombatantCoroutine


def nothing(**_kwargs: dict[str, Any]) -> CombatantCoroutine:
    """
    This opponent quite literally does nothing -- it always skips its turns, and any reactions.
    """
    while True:
        t = yield
        match t:
            case X.Turn() as turn:
                turn.end()
            case X.Reaction(X.AttackOfOpportunity() as aoo):
                aoo.skip()
            case X.GameEnd():
                return


class RandomKwargs(TypedDict): ...


def random(**_kwargs: dict[str, Any]) -> CombatantCoroutine:
    """
    This opponent picks randomly from legal actions.
    """
    mask: np.ndarray[tuple[int]]
    choice: int
    while True:
        legality = None
        t = yield
        match t:
            case X.GameEnd():
                return
            case X.Reaction(X.AttackOfOpportunity(actions=dnd_actions) as aoo):
                mask = np.array([True, *(a.is_available() for a in dnd_actions)])
                actions = np.arange(mask.shape[0])
                choice = np.random.choice(actions[mask])

                match choice:
                    case 0:
                        aoo.skip()
                        continue
                    case a:
                        legality = aoo.take(dnd_actions[a - 1].value())
                        assert not isinstance(legality, X.Illegal)
            case X.Turn(movement=movement, actions=dnd_actions) as turn:
                directions = movement.directions
                mask = np.array(
                    [True, *directions, *(a.is_available() for a in dnd_actions)]
                )
                actions = np.arange(mask.shape[0])
                choice = np.random.choice(actions[mask])

                match choice:
                    case 0:
                        turn.end()
                    case m if m < (1 + X.consts.SUPPORTED_MOVEMENT_DIRECTIONS):
                        legality = turn.move(m - 1)  # type: ignore
                        assert not isinstance(legality, X.Illegal)
                    case a:
                        action = dnd_actions[
                            a - 1 - X.consts.SUPPORTED_MOVEMENT_DIRECTIONS
                        ].value()
                        legality = turn.take(action)
                        assert not isinstance(legality, X.Illegal)


def human(**kwargs: dict[str, Any]) -> CombatantCoroutine:
    """
    A simple text interface for you to play along too.

    It is recommended that you enable debug mode for the engine by through
    `Game(..., debug=True)`.
    """
    name = kwargs.get("name", "Human")
    new_turn = True
    while True:
        t = yield
        match t:
            case X.GameEnd() as end:
                print("Did you win: ", end.won)
                return
            case X.Reaction(X.AttackOfOpportunity() as aoo):
                print(f"[{name}] AOO:")
                aoo_actions = aoo.actions
                print(f"  Actions (A0-{len(aoo_actions)}):", aoo_actions)
                print("  Skip (E)")

                aoo_report = None
                while True:
                    try:
                        match input("\nSelection: "):
                            case "E" | "e":
                                aoo.skip()
                            case a if "A" in a or "a" in a:
                                action_i = int(a[1:])
                                aoo_action = aoo_actions[action_i].value()
                                aoo_report = aoo.take(aoo_action)
                            case _:
                                continue
                    except ValueError:
                        continue
                    break

                if aoo_report is not None:
                    print(aoo_report)
            case X.Turn() as turn:
                if new_turn:
                    new_turn = False
                    print(f"[{name}] Turn:")
                else:
                    print()
                actions = turn.actions
                print("  Movement (M0-7):", turn.movement)
                print(f"  Actions  (A0-{len(actions)}):", actions)
                print("  End Turn (E)")
                print()

                report: X.Illegal | X.AttackReport | None = None
                while True:
                    try:
                        match input("Selection: "):
                            case a if "M" in a or "m" in a:
                                direction = int(a[1:])
                                report = turn.move(direction)  # type: ignore
                            case "E" | "e":
                                new_turn = True
                                turn.end()
                            case a if "A" in a or "a" in a:
                                action_i = int(a[1:])
                                action = actions[action_i].value()
                                report = turn.take(action)
                            case _:
                                continue
                    except ValueError:
                        continue
                    break

                if report is not None:
                    print(report)


DIRECTIONS = {
    (0.0, 1.0): 4,
    (1.0, 1.0): 3,
    (1.0, 0.0): 2,
    (1.0, -1.0): 1,
    (0.0, -1.0): 0,
    (-1.0, -1.0): 7,
    (-1.0, 0.0): 6,
    (-1.0, 1.0): 5,
}


def _displacement_to_direction(
    displ: np.ndarray[tuple[Literal[2]], np.dtype[np.float32]],
) -> int:
    sign: tuple[float, float] = np.sign(displ)  # type: ignore
    return DIRECTIONS[(sign[0], sign[1])]


def _find_target_index(view: X.View, target: str) -> int:
    return next(
        (i for i, e in enumerate(view.enemies) if e.name == target),
        -1,
    )


FollowCombatantCoroutine = Generator[bool | None, X.Turn | X.Reaction, None]


def follow(**kwargs: Any) -> CombatantCoroutine:
    """Follow the target around the arena, until the `until` function returns true. Then, execute one step of the `then` coroutine."""
    target_index = -1

    who: str = kwargs["who"]
    then: CombatantCoroutine = kwargs["then"]
    then.send(None)  # type: ignore
    while True:
        t = yield
        match t:
            case X.GameEnd():
                return
            case t:
                then.send(t)

        t = yield
        match t:
            case X.GameEnd():
                return
            case X.Turn() | X.Reaction():
                # Move closer towards the target.
                me: X.Me = t.me  # type: ignore
                view: X.View = me.view  # type: ignore

                if target_index == -1:
                    target_index = _find_target_index(view, who)

                    assert target_index != -1, (
                        "Target not found in the list of combatants."
                    )

                enemy = view.enemies[target_index]
                enemy_dist = me.distance_from(enemy)

                if enemy_dist <= X.consts.FEET_PER_SQUARE:
                    match t:
                        case X.Reaction(X.AttackOfOpportunity() as aoo):
                            aoo.skip()
                        case X.Turn() as turn:
                            turn.end()
                    continue
                else:
                    match t:
                        case X.Reaction(X.AttackOfOpportunity() as aoo):
                            # Cannot move on turn.
                            aoo.skip()
                        case X.Turn() as turn:
                            enemy_disp = me.displacement_from(enemy)

                            illegal: X.Illegal | None = X.Illegal("SQUARE_OCCUPIED")
                            direction = _displacement_to_direction(enemy_disp)
                            i = 0
                            for i in range(X.consts.SUPPORTED_MOVEMENT_DIRECTIONS):
                                illegal = turn.move(
                                    (direction + i)
                                    % X.consts.SUPPORTED_MOVEMENT_DIRECTIONS  # type: ignore
                                )

                                if illegal is None:
                                    break

                                if illegal.reason == "NO_MOVEMENT_LEFT":
                                    turn: X.Turn = yield  # type: ignore
                                    turn.end()
                                    break

                                turn: X.Turn = yield  # type: ignore


def attack(
    target: str, *, choice: Callable[[list[X.Attack]], X.Attack] = rand.choice
) -> CombatantCoroutine:
    """Attack a creature with the target name, if there is an eligible attack.

    This yields True if an action was taken, False otherwise.

    Use this coroutine in combination with the follow coroutine.
    """
    while True:
        t = yield
        if t is None:  # type: ignore
            continue
        attacks: list[X.Availability[X.Attack]] = t.actions[  # type: ignore
            X.consts.SUPPORTED_NON_ATTACK_ACTIONS :
        ]  # type: ignore
        available_attacks = [
            attack.value() for attack in attacks if attack.is_available()
        ]
        attacks_for_target = [
            attack for attack in available_attacks if attack.target.name == target
        ]

        # Either the target is not in range, or they don't exist, so do nothing.
        if len(attacks_for_target) == 0:
            continue

        attack = choice(attacks_for_target)
        result: X.Illegal | X.AttackReport = t.take(attack)  # type: ignore
        assert not isinstance(result, X.Illegal), "Attack agent did an illegal attack"


ModelCombatantCoroutine = Generator[
    int, np.ndarray[tuple[int], np.dtype[np.float32]], None
]
