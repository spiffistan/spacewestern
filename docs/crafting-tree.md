# Crafting & Building Dependency Tree

## Resource Gathering

```mermaid
graph LR
    TREE["Tree (chop, axe req.)"] -->|"4"| WOOD["Wood (logs)"]
    TREE -->|"3"| SCRAP["Scrap Wood"]
    TREE -->|"2"| FIBER["Fiber"]
    BUSH["Berry Bush"] -->|"3"| BERRIES["Berries"]
    CROP["Crop (harvest)"] -->|"2"| BERRIES
    CROP -->|"2"| FIBER
    DIG_CLAY["Clay Terrain (dig)"] -->|"4-6"| CLAY["Clay"]
    DIG_DIRT["Any Terrain (dig)"] -->|"1-2"| CLAY
    DEBRIS["Rock Debris"] -->|"1 / 4 w/ pick"| ROCK["Rock"]
    SHOVEL["Wooden Shovel"] -.->|"+2 yield"| DIG_CLAY
    SHOVEL -.->|"+2 yield"| DIG_DIRT
    PICK["Stone Pick"] -.->|"4x yield"| DEBRIS
    SPAWN["Spawn Area"] -.->|"scattered"| SCRAP
    SPAWN -.->|"scattered"| ROCK

    classDef src fill:#555,stroke:#333,color:#fff
    classDef res fill:#4a7c59,stroke:#2d5a3a,color:#fff
    classDef tool fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    classDef spawn fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class TREE,BUSH,CROP,DIG_CLAY,DIG_DIRT,DEBRIS src
    class WOOD,SCRAP,FIBER,BERRIES,CLAY,ROCK res
    class SHOVEL,PICK tool
    class SPAWN spawn
```

## Crafting Recipes

```mermaid
graph LR
    subgraph "Hand (no station)"
        R1["2 Rock + 1 Scrap"] --> AXE["Stone Axe"]
        R2["2 Rock + 1 Scrap"] --> PICK["Stone Pick"]
    end
    subgraph "Saw Horse"
        W1["1 Wood"] --> PLANKS["Planks x2"]
    end
    subgraph "Workbench"
        F1["4 Fiber"] --> ROPE["Rope"]
        S1["3 Scrap"] --> BUCKET["Wooden Bucket"]
        C1["2 Clay"] --> UNFIRED["Unfired Jug"]
        S2["2 Scrap + 1 Wood"] --> SHOVEL["Wooden Shovel"]
    end
    subgraph "Kiln"
        UNFIRED2["1 Unfired Jug"] --> JUG["Clay Jug"]
    end

    classDef input fill:#4a7c59,stroke:#2d5a3a,color:#fff
    classDef output fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    class R1,R2,W1,F1,S1,C1,S2,UNFIRED2 input
    class AXE,PICK,PLANKS,ROPE,BUCKET,UNFIRED,SHOVEL,JUG output
```

## First Night Survival (8-10 min)

```mermaid
graph TD
    A["Pick up sticks + rocks"] --> B["Hand-craft Stone Axe"]
    B --> C["Chop 2-3 trees"]
    C -->|"wood, scrap, fiber"| D["Dig earth nearby -> clay"]
    D --> E["Mud Walls 3x3 hut (2 clay each)"]
    C -->|"fiber"| F["Roof forms (1 fiber/tile)"]
    C -->|"1 wood"| G["Campfire inside (warmth + light)"]
    E --> H["Leave doorway gap"]
    F --> N1["Night 1: warm, safe, eat berries"]

    classDef phase0 fill:#6b7280,stroke:#4b5563,color:#fff
    classDef phase1 fill:#4a7c59,stroke:#2d5a3a,color:#fff
    classDef phase2 fill:#d97706,stroke:#92400e,color:#fff
    class A phase0
    class B,C,D phase1
    class E,F,G,H,N1 phase2
```

## Day 2+ Expansion

```mermaid
graph TD
    A["Hand-craft Stone Pick"] --> A2["Quarry rock (4x yield)"]
    B["Build Saw Horse (2 wood)"] --> C["Make Planks (1 wood -> 3)"]
    C --> D["Build Workbench (4 planks)"]
    D --> E["Craft Rope"]
    D --> F["Craft Shovel -> better digging"]
    E --> G["Build Bed (3 planks + 1 rope)"]
    E --> H["Build Wooden Door (future)"]
    E --> I["Build Well (4 wood + 2 rock + 1 rope)"]
    C --> J["Upgrade to Wood Walls (3 planks)"]
    A2 --> K["Stone Walls (3 rock)"]
    F --> L["Dig clay terrain -> Kiln (10 clay)"]

    classDef phase2 fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef phase3 fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class A,A2,B,C,D phase2
    class E,F,G,H,I,J,K,L phase3
    class B,B2,C,C2,D phase1
    class E,F,G,H phase2
    class I,J,K,L phase3
```

## Building: Structure

```mermaid
graph TD
    subgraph FLOORS
        RF["Rough Floor — 1 wood"]
        WF["Wood Floor — 2 planks"]
        SF["Stone Floor — 2 rock"]
    end
    subgraph WALLS["Walls (thin by default)"]
        MW["Mud Wall — 2 clay (cheapest!)"]
        WW["Wood Wall — 2 wood (logs)"]
        SW["Stone Wall — 3 rock"]
        GW["Glass — 2 rock"]
        IW["Insulated — 2 clay + 2 planks"]
        STW["Steel Wall — 4 rock"]
        DW["Diagonal — 2 rock"]
    end
    subgraph FURNITURE
        BN["Bench — 2 planks"]
        BD["Bed — 3 planks + 1 rope"]
        CR["Crate — 2 planks"]
    end
    subgraph ROOF
        TH["Thatch Roof — 1 fiber/tile (auto)"]
    end

    classDef item fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef cheap fill:#4a7c59,stroke:#2d5a3a,color:#fff
    class RF,WF,SF,WW,SW,GW,IW,STW,DW,BN,BD,CR item
    class MW,TH cheap
```

## Building: Utilities & Lighting

```mermaid
graph TD
    subgraph UTILITIES
        WL["Well — 3 wood + 2 rock + 1 rope"]
        FP["Campfire — 1 wood"]
        CP["Compost — 1 wood"]
        CN["Cannon — 6 rock"]
    end
    subgraph LIGHTING
        WT["Wall Torch — 1 wood"]
        FL["Floor Lamp — 1 plank"]
        CL["Ceiling Light — 1 plank"]
        WL2["Wall Lamp — 1 plank"]
        FD["Floodlight — 2 rock"]
        TL["Table Lamp — free"]
    end

    classDef item fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef free fill:#6b7280,stroke:#4b5563,color:#fff
    class WL,FP,CP,CN,WT,FL,CL,WL2,FD item
    class TL free
```

## Building: Power & Piping

```mermaid
graph TD
    subgraph POWER
        SO["Solar — 2 planks + 1 rock"]
        BS["Battery S — 1 plank + 1 rock"]
        BM["Battery M — 2 planks + 2 rock"]
        BL["Battery L — 3 planks + 3 rock"]
        WT["Wind Turbine — 2 wood + 2 planks + 1 rope"]
    end
    subgraph GAS["Gas Piping"]
        GP["Pump — 1 plank + 1 rock"]
        TK["Tank — 2 planks + 1 rock"]
        FN["Fan — 1 plank + 1 rock"]
        PB["Pipe / Valve / Outlet — free"]
    end
    subgraph LIQ["Liquid Piping"]
        LP["Liquid Pipe — 1 clay"]
        LI["Intake / Output — 1 clay"]
        LM["Liquid Pump — 1 plank + 1 rock"]
    end
    subgraph FREE_ELEC["Free (time only)"]
        WR["Wire / Bridge"]
        SW["Switch / Dimmer / Breaker"]
    end

    classDef item fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef free fill:#6b7280,stroke:#4b5563,color:#fff
    class SO,BS,BM,BL,WT,GP,TK,FN,LP,LI,LM item
    class PB,WR,SW free
```
