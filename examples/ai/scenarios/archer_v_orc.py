import numpy as np
import gymnasium as gym
from sb3_contrib.common.wrappers import ActionMasker

from xander import templating as T, Arena

from xander.ai.env import GameInit, XanderEnv
from xander.ai.opponents import random
from xander.ai.wrappers.sparse import Sparse

orc = T.Creature.load_json("./creatures/orc.json")
archer = T.Creature.load_json("./creatures/archer.json")
game: GameInit = {
    "arena": Arena(40, 40),
    "debug": False,
    "opponents": [
        {
            "controller": random,
            "name": "Orc",
            "position": "random",
            "seed": "random",
            "template": orc,
        },
    ],
    "rl_agent": {
        "controller": "agent",
        "name": "RL",
        "position": "random",
        "seed": "random",
        "template": archer,
    },
}

env: gym.Env[np.ndarray, int] = Sparse(XanderEnv(game), time_penalty=-0.5)
ENV = ActionMasker(env, "action_mask")
