---
"Dungeons & Dragons, Graphs & Hierarchies: Graph-based, Hierarchical Reinforcement Learning Agents Learn to Battle in D&D Combat"
---

Wider themes:
- hierarchical RL methods and coroutines are representing the same concepts:
    Sequences of steps that can be composed 


# 1. Introduction
- What is reinforcement learning, briefly.
- History of Reinforcement learning being used to play games, briefly mention other uses such as robotics.
- Introduce D&D. Why D&D? It's super hard! And the "next frontier" for RL. It's a super-hard game with many challenges to make it work in an RL context:
    - Action spaces vast
    - State space vast
    - Reward-sparse
- Introduce hierarchical techniques as a potential avenue to explore, and link the idea of temporal abstraction to how humans play D&D:
    - We think of *where* we wish to go, rather than each minute step (left, then up).
    - 
## 1.1 Aims
1. Present Dungeons & Dragons as a novel, exciting frontier for reinforcement learning research.
2. Improve on the ergonomics of the environment from our previous work by implementing D&D combat as cooperative multitasking process.
3. Implement agents using hierarchical techniques, evaluating their performance across multiple realistic D&D combat scenarios.
## 1.2 Introduction to Combat in Dungeons & Dragons
- @ Introduce notation and conventions.
    - To disambiguate terms from D&D and the RL literature such as 'action', throughout the rest of this paper, D&D-specific terms such as Turn, Reaction, Attack, and Action will be capitalised.   
    - Introduce the flow diagram with the blocks, arrows, and letters:
        - `T` for Turn
        - `R` for Reaction
    - Examples of stat blocks
    
    - A turn
        - Movement
        - Actions:
            - Attack (usually 1/turn, but occasionally can be more)
            - Dodge
            - Dash
            - Disengage
    - Reactions:
        - Attacks of Opportunity

    
    - Quick combat example.
## 1.3 Reinforcement Learning 
### 1.3.1 Markov Decision Processes
- What is an MDP?
- D&D is likely a Partially Observable Markov Decision Process.
    - Why?
### 1.3.2 Hierarchical Methods 
# 2. Related Work
## 2.1 RL Techniques used
## 2.2 D&D in AI in General
## 2.3 D&D in RL
## 2.4 Our previous work
Talk about some of the (many) flaws in previous work:
- No action masking
- ...
# 3. Methodology
## 3.1 The Environment
Brief overview again of how the environment is made.

- Rust-based, with Python bindings.
- Explain how it is modular (even more than before!)
### 3.1.2 Dungeons & Dragons Combat as a Cooperative Multitasking Process

- In the revised version of the environment, we treat Combat as a cooperative multitasking process, where the environment itself, and each agent is represented as a coroutine. Coroutines can be thought of as a more general form of a subroutine, where its execution can be suspended whilst another can proceed, before ceding control back to the original [1].

- Explain briefly how the above works in Python (Generators) and Rust (Await/Async):
    - Each `.await` in Rust means a possible call to Python.
    - Each `yield` in Python is a guaranteed call to the environment.

- The environment coordinates the combat according to initiative order, by calling each agent when it needs to make a decision (such as a Turn, or Reaction). The agent will then select an action, before yielding control to the environment. Then, the agent's coroutine is suspended and control flow is returned to the environment. \[Fig. Example\]

- Often, turns are composed of multiple time steps, where agents can chose one particular action (for example, moving one square, attacking), before ending their turn with the special &gt;End&lt; action.


- This is similar to the usual training loops for `gymnasium` environments, with the major difference being using `yield` instead of `Env.step()` -- an agent during training has to wait for its turn to then take its action (and perform back-propagation). \[Example as a listing:\]
```python
obs, info = yield
done = False
while not done:
    logits = model(obs)
    if random() >= EPSILON:
        action = choice(logits, mask=obs.possible_actions())
    else:
        action = argmax(logits, mask=obs.possible_actions())
    
    # Cedes back control to the environment
    obs.perform(action)
    
    # Cede control to the environment, and wait until the agent's next action.
    next_obs, reward, terminated, truncated, info = yield
    done = terminated or truncated
    obs = next_obs
```
- @ Say that this was a goal of the redesign, and that previously it was slightly more annoying and less elegant.

[1] https://dl.acm.org/doi/10.1145/366663.366704
## 3.2 Dungeons & Dragons Combat as a Sequential Decision Problem

## 3.3 Experiments

Outline the scenarios:
- Rat Battle (equal strength characters)
- Archer vs. Straahd (archer must stay away from Straahd)
- ...

Explain what strategies humans may implement in those scenarios.

# 4. Results

# 5. Discussion

Did we see any interpretable strategy in those scenarios?

- Credit assignment problems resolved?

# 5.1. Future Work
- More work on the environment:
    - Extending it to more actions
- Multi-agent learning
- ...

# 6. Conclusion