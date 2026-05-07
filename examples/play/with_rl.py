from typing import Any

from sb3_contrib import MaskablePPO
from xander.ai.env import GameInit
from xander.ai.utils import V1
from xander.ai.env import make_game
from xander.pyutils import CombatantCoroutine
from xander import templating as T, Arena
from xander.ai.opponents import human, random


giant_rat = T.Creature.load_json("../ai/creatures/giant_rat.json")


def with_model(**kwargs: Any) -> CombatantCoroutine:
    model: MaskablePPO = kwargs["model"]  # type: ignore
    while True:
        event = yield
        obs, mask, _ = V1.translate_state(event)
        action, _ = model.predict(obs, action_masks=mask)  # type: ignore
        res = V1.translate_action(event, action, model.observation_space, log=True)  # type: ignore

        match res:
            case (_, *_):
                return
            case _:
                pass
    ...


# Download from my HuggingFace! sammy99jsp/Xander
model = MaskablePPO.load("./ppo_play_preview.zip", device="cpu")  # type: ignore

game: GameInit = {
    "arena": Arena(50, 50),
    "debug": True,
    "opponents": [
        {
            "controller": with_model,
            "name": "AI",
            "position": "random",
            "seed": "random",
            "template": giant_rat,
            "kwargs": {"model": model},
        },
        {
            "controller": random,
            "name": "Random",
            "position": "random",
            "seed": "random",
            "template": giant_rat,
            "kwargs": {"model": model},
        },
    ],
    "rl_agent": {
        "controller": "agent",
        "name": "Human",
        "position": "random",
        "seed": "random",
        "template": giant_rat,
    },
}


g, _ = make_game(game, human())
g.start()
