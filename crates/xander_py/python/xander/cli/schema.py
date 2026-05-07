from .. import xander as X
from typing import Annotated, Optional

import typer


schema = typer.Typer()


@schema.command("creature")
def creature(
    path: Annotated[
        Optional[str],
        typer.Argument(
            help="Path to output file.",
            path_type=str,
            file_okay=True,
            dir_okay=False,
            exists=False,
        ),
    ] = None,
):
    match path:
        case None:
            print(X.schema.creature())
        case path:
            X.schema.creature(open(path, "w"))


@schema.command("game")
def game(
    path: Annotated[
        Optional[str],
        typer.Argument(
            help="Path to output file.",
            path_type=str,
            file_okay=True,
            dir_okay=False,
            exists=False,
        ),
    ] = None,
):
    match path:
        case None:
            print(X.schema.game())
        case path:
            X.schema.game(open(path, "w"))
