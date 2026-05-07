import numpy as np
import gymnasium as gym
from sb3_contrib.common.wrappers import ActionMasker

from xander import templating as T, Arena

from xander.ai.env import XanderEnv, GameInit
from xander.ai.opponents import random
from xander.ai.wrappers.sparse import Sparse


rat = T.Creature.load_json("./creatures/rat.json")
game: GameInit = {
    "arena": Arena(40, 40),
    "debug": False,
    "opponents": [
        {
            "controller": random,
            "name": "Rat",
            "position": "random",
            "seed": "random",
            "template": rat,
        },
    ],
    "rl_agent": {
        "controller": "agent",
        "name": "RL",
        "position": "random",
        "seed": "random",
        "template": rat,
    },
}

env: gym.Env[np.ndarray, int] = Sparse(XanderEnv(game), time_penalty=-0.5)
ENV = ActionMasker(env, "action_mask")
