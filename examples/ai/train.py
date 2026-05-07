import argparse

import numpy as np
import tqdm
import gymnasium as gym

from sb3_contrib import MaskablePPO
from stable_baselines3 import DQN
from stable_baselines3.common.base_class import BaseAlgorithm
from stable_baselines3.common.logger import configure

from scenarios.archer_v_orc import ENV as ARCHER_V_ORC
from scenarios.giant_rat_brawl import ENV as GIANT_RAT_BRAWL
from scenarios.giant_rat_v_hunter import ENV as GIANT_RAT_V_HUNTER
from scenarios.rat_duel import ENV as RAT_DUEL
from scenarios.swordsman_v_orc import ENV as SWORDSMAN_V_ORC

SCENARIOS = {
    "ARCHER_V_ORC": ARCHER_V_ORC,
    "GIANT_RAT_BRAWL": GIANT_RAT_BRAWL,
    "GIANT_RAT_V_HUNTER": GIANT_RAT_V_HUNTER,
    "RAT_DUEL": RAT_DUEL,
    "SWORDSMAN_V_ORC": SWORDSMAN_V_ORC,
}

MODELS: dict[str, type[BaseAlgorithm]] = {"PPO": MaskablePPO, "DQN": DQN}


def train_scenario(
    scenario: str,
    model: BaseAlgorithm,
    *,
    total_steps: int = 2_000_000,
    steps_chunks: int = 50_000,
):
    model.set_logger(
        configure(folder=f"./logs/{scenario}", format_strings=["stdout", "csv"])
    )

    bar = tqdm.tqdm(total=total_steps)
    bar.display()

    for i in range(total_steps // steps_chunks):
        model = model.learn(  # type: ignore
            total_timesteps=steps_chunks, progress_bar=True, reset_num_timesteps=False
        )
        model.save(f"checkpoints/{scenario}/{steps_chunks * (i + 1):06}.zip")
        bar.update(steps_chunks)


parser = argparse.ArgumentParser(
    prog="Scenario Trainer",
    description="Trains the Example Scenarios for XanderEnv",
)

parser.add_argument("--scenario", choices=SCENARIOS.keys(), required=True)
parser.add_argument("--model", choices=MODELS.keys(), required=True)
parser.add_argument("--total-steps", default=2_000_000)
parser.add_argument("--checkpoint-every", default=50_000)

if __name__ == "__main__":
    args = parser.parse_args()

    model_ty = args.model
    slug = f'{args.scenario}-{model_ty}'
    env: gym.Env[np.ndarray, int] = SCENARIOS[args.scenario]  # type: ignore
    total_steps: int = args.total_steps
    steps_chunks: int = args.checkpoint_every

    model = MODELS[model_ty]("MlpPolicy", env)  # type: ignore
    model.set_logger(
        configure(folder=f"./logs/{slug}", format_strings=["stdout", "csv"])
    )
    bar = tqdm.tqdm(total=total_steps)
    bar.display()

    for i in range(total_steps // steps_chunks):
        model = model.learn(  # type: ignore
            total_timesteps=steps_chunks, progress_bar=True, reset_num_timesteps=False
        )
        model.save(
            f"checkpoints/{slug}/{steps_chunks * (i + 1):06}.zip"
        )
        bar.update(steps_chunks)
