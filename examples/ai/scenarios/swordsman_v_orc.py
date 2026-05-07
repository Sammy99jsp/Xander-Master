import gymnasium as gym
import numpy as np

from sb3_contrib.common.wrappers import ActionMasker
from xander import templating as T, Arena

from xander.ai.env import XanderEnv, GameInit
from xander.ai.opponents import random
from xander.ai.wrappers.sparse import Sparse


orc = T.Creature.load_json("./creatures/orc.json")
swordsman = T.Creature.load_json("./creatures/swordsman.json")
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
        "template": swordsman,
    },
}


env: gym.Env[np.ndarray, int] = Sparse(XanderEnv(game), time_penalty=-0.5)
ENV = ActionMasker(env, "action_mask")
