from typing import Any

import numpy as np

import xander.xander as X
from xander.xander import consts  # type: ignore
from xander.pyutils import Event

SKIP = 1
NON_ATTACKS = consts.SUPPORTED_NON_ATTACK_ACTIONS
DIRECTIONS = consts.SUPPORTED_MOVEMENT_DIRECTIONS


class V1:
    @staticmethod
    def translate_state(event: Event) -> tuple[np.ndarray, np.ndarray, bool]:
        dirs: np.ndarray
        movement_used: list[float]

        terminated = False
        match event:
            case X.GameEnd() as end:
                terminated = True
                me = end.me
                dirs = np.zeros((DIRECTIONS,))
                actions_mask = np.zeros((me.len_actions,))
                movement_used = [0.0, 0.0]

            case X.Reaction(X.AttackOfOpportunity() as aoo):
                actions_mask = np.array([float(a.is_available()) for a in aoo.actions])

                movement_used = [0.0, 0.0]
                me = aoo.me

                dirs = np.zeros((8,))
            case X.Turn() as turn:
                actions_mask = np.array(
                    [float(a.is_available()) for a in turn.actions], dtype=np.float32
                )

                me = turn.me
                my_hp = me.hp

                movement = turn.movement
                dirs = movement.directions
                movement_used = [
                    my_hp[0] / my_hp[1],
                    movement.left / movement.speed,
                ]

        view = me.view
        dims = view.arena_dims
        mask = np.concat(
            [
                [float(not terminated)],  # End Turn / Skip
                dirs,
                actions_mask,
            ],
            dtype=np.float32,
        )

        state: np.ndarray = np.concat(
            [
                movement_used,
                *(
                    [e.hp[0] / e.hp[1], *(me.displacement_from(e) / dims)]
                    for e in view.enemies
                ),
            ],
            dtype=np.float32,
        )

        return state, mask, terminated

        ...

    @staticmethod
    def translate_action(
        event: Event, action: int, obs_space: Any, log: bool = False
    ) -> (
        tuple[np.ndarray, float, bool, bool, dict[str, Any]]
        | X.AttackReport
        | X.Illegal
        | None
    ):
        res: X.AttackReport | X.Illegal | None = None
        match event:
            case X.GameEnd() as report:
                return (
                    np.zeros(obs_space._shape),  # type: ignore
                    0.0,
                    True,
                    False,
                    {"won": report.won},
                )  # type: ignore
            case X.Reaction(X.AttackOfOpportunity() as aoo):
                attacks = aoo.actions
                match action:
                    case 0:
                        aoo.skip()
                    case a if a < (SKIP + DIRECTIONS):
                        # Trying to move in a reaction, shut that down...
                        res = X.Illegal("OUT_OF_TURN")
                    case a if a < (SKIP + DIRECTIONS + NON_ATTACKS):
                        # Trying to do a non-attack in an AOO, shut that down...
                        res = X.Illegal("OUT_OF_TURN")
                    case a:
                        aoo_chosen_action = attacks[a - (SKIP + DIRECTIONS)].value()

                        if log:
                            print("[RL]", aoo_chosen_action)

                        res = aoo.take(aoo_chosen_action)

                        if log:
                            print("[RL]", res)
            case X.Turn() as turn:
                match action:
                    case 0:
                        turn.end()
                    case m if m < (SKIP + DIRECTIONS):
                        direction = m - 1

                        if log:
                            print("[RL]", consts.DIRECTION_ARROW[direction])

                        res = turn.move(direction)  # type: ignore
                    case a:
                        dnd_actions = turn.actions
                        chosen_action = dnd_actions[a - (SKIP + DIRECTIONS)].value()

                        if log:
                            print("[RL]", chosen_action)
                        res = turn.take(chosen_action)

                        if log and res is not None:
                            print("[RL]", res)
        return res
