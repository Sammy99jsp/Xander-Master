from typing import Any, SupportsFloat

import gymnasium as gym
import numpy as np

from sb3_contrib.common.wrappers import ActionMasker
from xander import templating as T, Arena

from xander.ai.env import XanderEnv, GameInit
from xander.ai.opponents import attack, follow
from xander.ai.wrappers.sparse import Sparse


class Survival(gym.Wrapper[np.ndarray, int, np.ndarray, int]):
    env: XanderEnv
    _for: int
    _win: float
    _steps: int = 0

    def __init__(
        self,
        env: XanderEnv,
        *,
        for_steps: int = 200,
        win: float = 10.0,
    ):
        super().__init__(env)
        self.env = env
        self._for = for_steps
        self._win = win

    def step(
        self, action: int
    ) -> tuple[
        np.ndarray[tuple[Any, ...], np.dtype[Any]],
        SupportsFloat,
        bool,
        bool,
        dict[str, Any],
    ]:
        obs, reward, terminated, truncated, info = self.env.step(action)

        if terminated or truncated:
            return obs, reward, terminated, truncated, info

        self._steps += 1

        # If has survived for `self._for` timesteps, we call that a win!
        if self._steps >= self._for:
            return obs, (reward + self._win), True, True, info  # type: ignore

        return obs, reward, terminated, truncated, info

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        return super().reset(seed=seed, options=options)

    def action_mask(self, *_) -> np.ndarray:
        return self.env.action_mask()


hunter = T.Creature.load_json("./creatures/hunter.json")
giant_rat = T.Creature.load_json("./creatures/giant_rat.json")
game: GameInit = {
    "arena": Arena(200, 40),
    "debug": False,
    "opponents": [
        {
            "controller": follow,
            "name": "Hunter",
            "position": "random",
            "seed": "random",
            "template": hunter,
            "kwargs": {"who": "RL", "then": attack("RL")},
        },
    ],
    "rl_agent": {
        "controller": "agent",
        "name": "RL",
        "position": "random",
        "seed": "random",
        "template": giant_rat,
    },
}

env: gym.Env[np.ndarray, int] = Survival(
    Sparse(XanderEnv(game), time_penalty=0.1),  # type: ignore
    for_steps=200,
    win=10.0,
)
ENV = ActionMasker(env, "action_mask")
