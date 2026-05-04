import typing

from xander import (
    Attack,
    Game,
    Arena,
    Movement,
    Reaction,
    Turn,
    Agent,
    AttackOfOpportunity,
    Creature,
    AgentCoroutine,
    Illegal,
    AttackReport,
    Position,
    Action,
    Availability,
)


def do_nothing(name: str) -> AgentCoroutine:
    while True:
        t = yield
        match t:
            case Turn(actions=actions, movement=Movement() as movement) as turn:
                print(f"--- {name} ---")
                action = next(  # type: ignore
                    map(Availability.value, filter(Availability.is_available, actions)),  # type: ignore
                    None,
                )
                action = typing.cast(Action | None, action)

                if action is not None:
                    print(f" => Taking: {action}")
                    turn.take(action)
                else:
                    print(" => End Turn")
                    print()
                    turn.end()

            case Reaction(AttackOfOpportunity(attacks=attacks, target=target) as aoo):
                print(f"AOO on {target}!")
                mask = list(map(Availability.is_available, attacks))  # type: ignore
                print(mask)
                attack = next(  # type: ignore
                    map(Availability.value, filter(Availability.is_available, attacks)),  # type: ignore
                    None,
                )

                attack = typing.cast(Attack | None, attack)

                print(attack)

                if attack is not None:
                    report = aoo.attack(attack)
                    match report:
                        case Illegal() as illegal:
                            print("Did something illegal:", illegal)
                        case AttackReport(damage=d) as report:
                            print("Dealt", d, "damage to", target.name)
                            print(report)
                else:
                    aoo.skip()


def assert_turn(t: Turn | Reaction) -> Turn:
    assert isinstance(t, Turn)
    return t


def test_coroutine(name: str) -> AgentCoroutine:
    while True:
        t = yield
        print(f"--- {name} ---")
        # print(name, "is going UP!")
        turn = assert_turn(t)
        attack = next(
            (
                attack
                for action in turn.actions
                if action.is_available()
                and isinstance(attack := action.value(), Attack)
            ),
            None,
        )
        match attack:
            case None:
                print(" => Skipping!")
                print()
                turn.end()
            case Attack():
                print(f" => Taking: {attack}")
                report = turn.take(attack)
                print(f" => Report: {report}")



arena = Arena(30, 30)

starting_poses: list[Position] = []

# while len(starting_poses) < 2:
#     while (pos := arena.random_square()) in starting_poses:
#         pass

#     starting_poses.append(pos)

game = Game(arena)
game.join(
    Agent("Jerry", test_coroutine("Jerry")),
    Creature.test(),
    arena.square_at(10, 10),
)
game.join(
    Agent("Dale", do_nothing("Dale")),
    Creature.test(),
    arena.square_at(15, 15),
)
game.start()
