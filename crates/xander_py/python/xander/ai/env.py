import threading
from typing import Any, Literal, NotRequired, Protocol, SupportsFloat, TypedDict

import gymnasium as gym
import numpy as np
from xander.ai.utils import V1
from xander.xander import (
    Me,
    Game,
    Arena,
    Agent,
    Illegal,
    AttackReport,
    Position,
    GameEnd,
    templating as T,
    consts,
)

from xander.pyutils import CombatantCoroutine, Event

# Utilities

SKIP = 1
NON_ATTACKS = consts.SUPPORTED_NON_ATTACK_ACTIONS
DIRECTIONS = consts.SUPPORTED_MOVEMENT_DIRECTIONS


class CombatCoroutineFn(Protocol):
    def __call__(self, **kwargs: Any) -> CombatantCoroutine: ...


class AgentInit(TypedDict):
    template: T.Creature
    position: tuple[int, int] | Literal["random"]
    name: NotRequired[str]
    seed: int | Literal["random"]
    controller: Literal["agent"]
    kwargs: NotRequired[dict[str, Any]]


class CombatantInit(TypedDict):
    template: T.Creature
    position: tuple[int, int] | Literal["random"]
    name: NotRequired[str]
    seed: int | Literal["random"]
    controller: CombatCoroutineFn
    kwargs: NotRequired[dict[str, Any]]


class GameInit(TypedDict):
    arena: Arena
    opponents: list[CombatantInit]
    rl_agent: AgentInit
    debug: NotRequired[bool]


def make_game(
    init: GameInit, rl_routine: CombatantCoroutine, *, seed: int | None = None
) -> tuple[Game, Me]:
    arena = init["arena"]
    game = Game(arena, debug=init.get("debug", False))

    combatants: list[CombatantInit | AgentInit] = [init["rl_agent"], *init["opponents"]]

    positions: list[Position] = []

    # Deal with fixed positions first
    for c in (c for c in combatants if c["position"] != "random"):
        pos = arena.square_at(*c["position"])  # type: ignore
        if any(p == pos for p in positions):
            raise ValueError(
                "Two fixed position combatants are occupying the same square. Bailing!"
            )
        positions.append(pos)

    # Now allocate the random ones
    i = 0
    for c in (c for c in combatants if c["position"] == "random"):
        while True:
            pos = arena.random_square(seed=(seed + i) if seed is not None else None)
            if not any(p == pos for p in positions):
                positions.append(pos)
                break

            i += 1

    rl = init["rl_agent"]
    rl_agent = Agent(
        rl.get("name", "RL Agent"),
        rl_routine,
        seed=seed if seed is not None else rl["seed"],
    )
    rl_creature = rl["template"].make(game, name=rl.get("name"))
    me = game.join(rl_agent, rl_creature, positions[0])  # type: ignore

    for i, (opponent, pos) in enumerate(zip(init["opponents"], positions[1:])):
        kwargs = opponent.get("kwargs", {})
        name = opponent.get("name", f"{i + 1}")
        creature = opponent["template"].make(game, name=name)
        game.join(
            Agent(
                name,
                opponent["controller"](**kwargs),
                seed=seed if seed is not None else opponent["seed"],
            ),
            creature,
            pos,
        )

    return game, me


# --- Worker Thread ---
def _sync_routine(
    this: "XanderEnv",
    *,
    next: threading.Semaphore,
    wait: threading.Semaphore,
) -> CombatantCoroutine:
    """[WORKER THREAD] The coroutine which runs as proxy for the environment."""
    while True:
        t = yield

        if this._early_terminate:  # type: ignore
            return

        wait.release()
        this._event = t  # type: ignore
        next.acquire()
        if isinstance(t, GameEnd):
            return


def _run_game(this: "XanderEnv", wait: threading.Semaphore) -> None:
    """[WORKER THREAD] The main function which starts the game."""
    try:
        this._game.start()  # type: ignore
    except StopIteration as e:
        if not this._early_terminate:  # type: ignore
            raise e
    wait.release()


# --- /Worker Thread ---


class XanderEnv(gym.Env[np.ndarray, int]):
    """
    A synchronous wrapper over the Xander Engine's Python API as a Gymnasium environment.

    This uses a worker thread to run the main game in the background and blocks until it is the agent's turn.

    The environment returns reward = 0.0 constantly, so please use a wrapper.

    `info` can be one of the following:
    * { "illegal": str } => if the agent has played an illegal move.
    * { "won": bool } => if the game has ended
    * { "damage": int | None, "hit": bool, "to_hit": int }
    """

    _event: Event
    _restarting: bool = False
    _game: Game
    _me: Me
    _early_terminate: bool = False
    _thread: threading.Thread

    def __init__(
        self,
        init: GameInit,
    ) -> None:
        self._init = init

        # We annoying have to set this up here to get the canonical
        # number of actions/attacks we have.

        # Multithreading setup...
        self._next = threading.Semaphore(0)
        self._wait = threading.Semaphore(0)

        self._game, self._me = make_game(
            self._init,
            _sync_routine(self, next=self._next, wait=self._wait),  # type: ignore
        )

        self.action_space = gym.spaces.Discrete(
            n=SKIP + DIRECTIONS + self._me.len_actions
        )

        # fmt: off
        self.observation_space = gym.spaces.Box(
            -1.0, 1.0,
            shape=[2 + 3 * len(self._init["opponents"])],  # type: ignore
            dtype=np.float32,
        )
        # fmt: on

    def action_mask(
        self,
    ) -> np.ndarray[tuple[int], np.dtype[np.bool]]:
        _, mask, _ = V1.translate_state(self._event)
        return mask

    def _advance(
        self, res: GameEnd | Illegal | AttackReport | None
    ) -> tuple[np.ndarray, bool, dict[str, Any]]:
        self._wait.acquire()

        terminated = False
        info: dict[str, Any] = {}

        state, mask, terminated = V1.translate_state(self._event)
        info["mask"] = mask

        if isinstance(self._event, GameEnd):
            info["won"] = self._event.won

            # Help the rules engine out by returning
            # from the coroutine attached to this env.
            #
            # The coroutine will return (raising StopIteration)
            # which xander.py will graciously handle.
            #
            # All other official coroutines are well-behaved too,
            # so the game should end here.

            self._next.release()
            self._wait.acquire()

        match res:
            case None:
                pass
            case Illegal() as s:
                info["illegal"] = str(s)
            case AttackReport() as report:
                info["damage"] = (
                    report.damage.total() if report.damage is not None else None
                )
                info["hit"] = report.hit
                info["to_hit"] = report.to_hit.total()
            case GameEnd() as endgame:
                info["won"] = endgame.won

        return state, terminated, info

    def step(
        self, action: int
    ) -> tuple[np.ndarray, SupportsFloat, bool, bool, dict[str, Any]]:
        res = V1.translate_action(self._event, action, self.observation_space)
        if isinstance(res, tuple):
            return res

        self._next.release()
        obs, terminated, info = self._advance(res)

        return obs, 0.0, terminated, False, info

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed, options=options)
        self._early_terminate = True

        # Ensure the thread is actually done by joining.
        # In theory, it should be by this point.
        if self._restarting:
            self._next.release()
            self._thread.join()
            del self._thread

            del self._next
            del self._wait
            del self._me
            del self._game

            # Multithreading setup...
            self._next = threading.Semaphore(0)
            self._wait = threading.Semaphore(0)

            self._game, self._me = make_game(
                self._init,
                _sync_routine(self, next=self._next, wait=self._wait),  # type: ignore
                seed=seed,
            )
        else:
            self._restarting = True

        # Add the rest of the combatants
        self._early_terminate = False
        self._thread = threading.Thread(
            target=lambda: _run_game(self, self._wait), daemon=True
        )

        self._thread.start()

        obs, _, info = self._advance(None)
        return obs, info
