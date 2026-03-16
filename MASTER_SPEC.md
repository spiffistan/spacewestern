Rayworld is a raytraced rimworld-esque game.

- it is primaryly block-based like rimworld, but one can place certain things on a lower grid (maybe 4x4x4 or more within a block?)
- it runs extremely fast natively and on web (in ticks per second it is very optimized)
- it is always only raytraced
- it has a top-down view that looks sort of like rimworld (maybe like a fisheye or a slight perspective)
- it has many of the same overarching main concepts like rimworld has
  - story driven "no endgame" gameplay
  - resource gathering
  - pawn management (we call them plebs)
  - etc.

- it has an extremely fast fluid simulation engine for simulating air in the world (and other gases)
  - it may be neccessary for the fluid sim to be mostly 2d.
  - it should take into consideration the interaction of gases and heat
  - ideally it should have phase interaction as well
  - ideally several gases should interact with each other (e.g. plebs produce co2 but consume oxygen, vice versa for trees and plants etc)
- it has an extremely fast physics engine that interacts with the fluid simulation engine
- it has a robust lighting system that can interact with the physics system and the fluid system
- the blocks, the fluid and the physics all interact with the plebs. The entire world is integrated.

- for a prototype a small simulation of say 100x50 should be made, but the real simulation should be made much larger (take this into consideration, at least 500x500 in production)

- it must be possible to import assets into the world from blender. These should be quite simple to match the rimworld style.

- for the prototype a gameplay of survival in the wilderness is the goal, taking into consideration weather and temperature

