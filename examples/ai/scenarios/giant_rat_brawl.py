import numpy as np
import gymnasium as gym
from sb3_contrib.common.wrappers import ActionMasker

from xander import templating as T, Arena

from xander.ai.env import GameInit, XanderEnv
from xander.ai.opponents import random
from xander.ai.wrappers.sparse import Sparse


giant_rat = T.Creature.load_json("./creatures/giant_rat.json")
game: GameInit = {
    "arena": Arena(40, 40),
    "debug": False,
    "opponents": [
        {
            "controller": random,
            "name": "1",
            "position": "random",
            "seed": "random",
            "template": giant_rat,
        },
        {
            "controller": random,
            "name": "2",
            "position": "random",
            "seed": "random",
            "template": giant_rat,
        },
        {
            "controller": random,
            "name": "3",
            "position": "random",
            "seed": "random",
            "template": giant_rat,
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

env: gym.Env[np.ndarray, int] = Sparse(XanderEnv(game), time_penalty=-0.5)
ENV = ActionMasker(env, "action_mask")
