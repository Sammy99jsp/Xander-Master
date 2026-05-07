from typing import Any, SupportsFloat

import gymnasium
import numpy as np
from xander.ai.env import XanderEnv


class Sparse(gymnasium.Wrapper[np.ndarray, int, np.ndarray, int]):
    _time_penalty: float
    _win: float
    _lose: float
    _illegl: float

    def __init__(
        self,
        env: XanderEnv,
        *,
        time_penalty: float = -0.5,
        win: float = 10.0,
        lose: float = -10.0,
        illegal: float = -0.5,
    ):
        super().__init__(env)
        self.env = env
        self._time_penalty = time_penalty
        self._win = win
        self._lose = lose

    def step(
        self, action: int
    ) -> tuple[
        np.ndarray,
        SupportsFloat,
        bool,
        bool,
        dict[str, Any],
    ]:
        obs, reward, terminated, truncated, info = self.env.step(action)

        reward += self._time_penalty

        match info:
            case {"won": v, **_k}:
                reward += self._win if v else self._lose

        return obs, reward, terminated, truncated, info

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        return super().reset(seed=seed, options=options)

    def action_mask(self, *_) -> np.ndarray:
        return self.env.action_mask()
