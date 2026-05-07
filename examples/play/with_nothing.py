from xander import (
    Game,
    Arena,
    Agent,
    templating as T,
    ai as A,
)


giant_rat = T.Creature.load_json("../ai/creatures/giant_rat.json")
rat_archer = T.Creature.load_json("../ai/creatures/archer.json")

arena = Arena(120, 120)
game = Game(arena, debug=True)

game.join(
    Agent("Giant Rat", A.opponents.nothing(), seed=0),
    giant_rat.make(game, name="Giant Rat"),
    arena.square_at(10, 10),
)

player = game.join(
    Agent("Player", A.opponents.human(), seed="random"),
    rat_archer.make(game, name="Human"),
    arena.square_at(80, 80),
)

game.start()
