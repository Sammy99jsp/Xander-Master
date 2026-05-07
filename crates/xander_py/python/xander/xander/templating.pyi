from io import TextIOWrapper
from typing import overload

from .. import xander as X

class Creature:
    @overload
    @staticmethod
    def load_json(path: str) -> Creature: ...
    @overload
    @staticmethod
    def load_json(file: TextIOWrapper) -> Creature: ...
    def make(self, game: X.Game, *, name: str | None = None) -> X.Creature: ...
