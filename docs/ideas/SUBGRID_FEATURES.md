# Sub-Grid Feature Ideas

Ideas that build on the 4×4 sub-grid architecture (DN-003, DN-004). All of these turn a tile from "one thing" into "a composition of features" by letting different sub-cells serve different purposes.

## Alcoves and Niches

Wall thickness varies along the edge. 3 sub-cells are thickness-2, but the middle sub-cell is thickness-1, creating a recessed niche. A bookshelf or fireplace built into the wall. Visually rich, still one tile.

## Half-Height Walls and Railings

Not just thinner but shorter. A fence or railing at sub-height — you can see over it (doesn't block vision) but can't walk through (blocks pathfinding). Porches, balconies, animal pens. The sub-grid stores thickness AND height per edge.

## Pipes and Wires Inside Walls

Currently pipes/wires occupy their own tile. With wall thickness, they could route through the wall's sub-cells — hidden infrastructure. A pipe runs through a thickness-2 wall, invisible, not eating floor space.

## Floor Layers

The sub-grid could distinguish floor material per sub-cell — a rug on wood floor, a hearth mat in front of a fireplace, a doormat at the entrance. Not a new block type, just a sub-grid overlay on the floor tile.

## Curved Walls

With 4×4 resolution, approximate curves by varying which sub-cells are wall per tile. A round room is a series of tiles where the wall edge steps inward/outward by one sub-cell per tile. Crude at 4×4, but recognizable.
