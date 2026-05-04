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
    AgentCoroutine,
    Illegal,
    AttackReport,
    Position,
    Action,
    Availability,
    GameEnd,
)


def dale(name: str = "Dale") -> AgentCoroutine:
    new_turn = True
    while True:
        t = yield
        if new_turn:
            print(f"--- {name} ---")
            new_turn = False
        match t:
            case Turn(actions=actions, movement=Movement() as movement) as turn:
                print(" => Turn")
                print(" => ", movement)
                print(" => ", actions)
                action = next(  # type: ignore
                    map(Availability.value, filter(Availability.is_available, actions)),  # type: ignore
                    None,
                )
                action = typing.cast(Action | None, action)

                if action is not None:
                    print(f" => Taking: {action}")
                    l = turn.take(action)
                    print(l)
                else:
                    print(" => End Turn")
                    print()
                    turn.end()
                    new_turn = True

            case GameEnd(won=won):
                print(f" ?? Did {name} win? {won}")

            case Reaction(AttackOfOpportunity(attacks=attacks, target=target) as aoo):
                print(f" => AOO on {target}!")
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
                            print("      Did something illegal:", illegal)
                        case AttackReport(damage=d) as report:
                            print("        Dealt", d, "damage to", target.name)
                            print(report)
                else:
                    aoo.skip()


def assert_turn(t: Turn | Reaction | GameEnd) -> Turn:
    assert isinstance(t, Turn)
    return t


def jerry(name: str = "Jerry") -> AgentCoroutine:
    new_turn = True
    while True:
        t = yield
        if new_turn:
            print(f"--- {name} ---")
            new_turn = False
        # print(name, "is going UP!")

        match t:
            case GameEnd(won=won):
                print(f" ?? Did {name} win? {won}")
            case Reaction(AttackOfOpportunity() as aoo):
                print("Skipping aoo")
                aoo.skip()
            case Turn(movement=movement, actions=actions) as turn:
                print(" => Turn")
                print(" => ", movement)
                print(" => ", actions)
                turn = assert_turn(t)
                attack = next(
                    (
                        attack
                        for action in actions
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
                        new_turn = True
                    case Attack():
                        print(f" => Taking: {attack}")
                        report = turn.take(attack)
                        print(f" => Report: {report}")


arena = Arena(30, 30)

starting_poses: list[Position] = []

game = Game(arena)
creature_jerry = game.load_creature_json("./test.json", name="Jerry")
game.join(
    Agent("Jerry", jerry()),
    creature_jerry,
    arena.square_at(10, 10),
)

creature_dale = game.load_creature_json("./test.json", name="Dale")
game.join(
    Agent("Dale", dale()),
    creature_dale,
    arena.square_at(15, 15),
)
game.start()
