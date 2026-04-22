# Crafting & Building Dependency Tree

## Primitive Tool Chain (Day 1)

```mermaid
graph LR
    ROCK["Rock (ground)"] -->|"pick up"| HAMMERSTONE["Hammerstone"]
    ROCK -->|"1 rock, 3s"| BLADE["Stone Blade"]
    STICK["Sticks (gather)"] -->|"1 stick, 2s"| DIGSTICK["Digging Stick"]

    ROCK2["1 Rock"] --> AXE["Stone Axe"]
    STICK2["1 Stick"] --> AXE
    FIBER["1 Fiber"] --> AXE

    ROCK3["1 Rock"] --> PICK["Stone Pick"]
    STICK3["1 Stick"] --> PICK
    FIBER2["1 Fiber"] --> PICK

    ROCK4["1 Rock"] --> KNIFE["Hunting Knife"]
    STICK4["1 Stick"] --> KNIFE
    FIBER3["1 Fiber"] --> KNIFE

    FIBER4["3 Fiber"] --> BELT["Fiber Belt"]

    classDef raw fill:#4a7c59,stroke:#2d5a3a,color:#fff
    classDef tool fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    classDef equip fill:#3b6b8c,stroke:#2a4f6a,color:#fff
    class ROCK,ROCK2,ROCK3,ROCK4,STICK,STICK2,STICK3,STICK4,FIBER,FIBER2,FIBER3,FIBER4 raw
    class HAMMERSTONE,BLADE,DIGSTICK,AXE,PICK,KNIFE tool
    class BELT equip
```

## Resource Gathering

```mermaid
graph LR
    TREE["Tree (chop)"] -->|"axe req."| LOG["Log"]
    TREE -->|"3"| STICKS["Sticks"]
    TREE -->|"2"| FIBER["Fiber"]
    TREE -->|"no axe"| STICKS2["5-10 Sticks + Fiber"]
    BUSH["Berry Bush"] -->|"harvest"| BERRIES["Berries"]
    DUSTW["Dustwhisker"] -->|"harvest"| FIBER2["Fiber"]
    SALT["Saltbrush"] -->|"harvest"| SALT2["Salt"]
    REED["Hollow Reed"] -->|"harvest"| REED2["Reed Stalk"]
    THORN["Thornbrake"] -->|"harvest"| THORN2["Thorns"]
    DUSK["Duskbloom"] -->|"harvest"| NECTAR["Nectar"]
    DUSK -->|"harvest"| PETALS["Dried Petals"]
    ROCK_TILE["Rock (mine)"] -->|"pick speeds"| ROCK["Rock"]
    CLAY_TILE["Clay Terrain"] -->|"dig, shovel helps"| CLAY["Clay"]
    CRATE["Salvage Crate"] -->|"open"| SUPPLIES["Mixed supplies"]

    classDef src fill:#555,stroke:#333,color:#fff
    classDef res fill:#4a7c59,stroke:#2d5a3a,color:#fff
    classDef special fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class TREE,BUSH,DUSTW,SALT,REED,THORN,DUSK,ROCK_TILE,CLAY_TILE src
    class LOG,STICKS,STICKS2,FIBER,FIBER2,BERRIES,SALT2,REED2,THORN2,NECTAR,PETALS,ROCK,CLAY res
    class CRATE,SUPPLIES special
```

## Crafting Stations Progression

```mermaid
graph TD
    HANDS["Bare Hands"] -->|"day 1"| BENCH["Rough Bench<br/>3 sticks + 1 fiber"]
    BENCH -->|"+ hammerstone"| KNAPPING["knapping capability"]
    BENCH -->|"+ knife"| CUTTING["cutting capability"]

    AXE["Stone Axe"] -->|"chop trees"| LOGS["Logs"]
    LOGS --> SAWHORSE["Saw Horse<br/>2 logs"]
    SAWHORSE -->|"1 log"| PLANKS["Planks x2"]

    PLANKS --> TABLE["Plank Table<br/>2 planks + 2 sticks<br/>3 tool slots"]
    PLANKS --> STOOL["Stool<br/>2 sticks + 1 fiber"]
    TABLE -->|"+ multiple tools"| WORKSHOP["Workshop"]

    PLANKS --> WORKTABLE["Work Table<br/>3 planks + 1 rope<br/>3 tool slots"]
    PLANKS --> LONGTABLE["Long Table<br/>5 planks + 2 rope<br/>5 tool slots"]
    PLANKS --> CORNER["Corner Table<br/>4 planks + 1 rope<br/>L-shaped, 3 slots"]

    CLAY -->|"10 clay + 4 rock"| KILN["Kiln"]

    classDef tier0 fill:#6b7280,stroke:#4b5563,color:#fff
    classDef tier1 fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    classDef tier2 fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef tier3 fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class HANDS,KNAPPING,CUTTING tier0
    class BENCH,AXE,LOGS,SAWHORSE,PLANKS,STOOL tier1
    class TABLE,WORKTABLE,LONGTABLE,CORNER,WORKSHOP tier2
    class KILN tier3
```

## Surface + Tool = Capability

```mermaid
graph LR
    subgraph "Tools (placed on surface)"
        HAMMER["Hammerstone"]
        WHET["Whetstone"]
        KNIFE["Knife/Blade"]
        MORTAR["Mortar & Pestle"]
        SPINDLE["Drop Spindle"]
        NEEDLE["Needle & Awl"]
    end

    subgraph "Capabilities"
        HAMMER -->|"100%"| KNAP["knapping"]
        HAMMER -->|"60%"| SHARP["sharpening"]
        WHET -->|"100%"| SHARP
        KNIFE -->|"100%"| CUT["cutting"]
        KNIFE -->|"100%"| CARVE["carving"]
        MORTAR -->|"100%"| GRIND["grinding"]
        SPINDLE -->|"100%"| SPIN["spinning"]
        NEEDLE -->|"100%"| SEW["sewing"]
    end

    classDef tool fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    classDef cap fill:#4a7c59,stroke:#2d5a3a,color:#fff
    class HAMMER,WHET,KNIFE,MORTAR,SPINDLE,NEEDLE tool
    class KNAP,SHARP,CUT,CARVE,GRIND,SPIN,SEW cap
```

## Furniture Progression

```mermaid
graph TD
    subgraph "Tier 0: Natural"
        FLATROCK["Flat Rock<br/>1 slot, 0.7x"]
        STUMP["Log Stump<br/>1 slot, 0.7x"]
        WRECK["Wreck Panel<br/>2 slots, 0.8x"]
    end

    subgraph "Tier 1: Primitive"
        RBENCH["Rough Bench<br/>2 slots, 0.85x"]
        RSTOOL["Rough Stool<br/>+5% craft speed"]
        DRYrack["Drying Rack"]
    end

    subgraph "Tier 2: Constructed"
        PTABLE["Plank Table<br/>3 slots, 1.0x"]
        CHAIR["Chair<br/>+10% craft speed"]
        LTABLE["Long Table<br/>5 slots, 1.0x"]
        CTABLE["Corner Table<br/>3 slots, L-shape"]
        SHELF["Wall Shelf<br/>2 storage"]
        RACK["Tool Rack<br/>4 display"]
    end

    subgraph "Tier 3: Heavy"
        STONE["Stone Bench<br/>4 slots, 1.1x"]
        METAL["Metal Bench<br/>4 slots, 1.2x"]
    end

    FLATROCK -.-> RBENCH
    RBENCH -.-> PTABLE
    PTABLE -.-> LTABLE
    PTABLE -.-> STONE
    STUMP -.-> RSTOOL
    RSTOOL -.-> CHAIR

    classDef t0 fill:#6b7280,stroke:#4b5563,color:#fff
    classDef t1 fill:#7a5c3a,stroke:#5a3d1e,color:#fff
    classDef t2 fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef t3 fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class FLATROCK,STUMP,WRECK t0
    class RBENCH,RSTOOL,DRYrack t1
    class PTABLE,CHAIR,LTABLE,CTABLE,SHELF,RACK t2
    class STONE,METAL t3
```

## First 30 Days Overview

```mermaid
graph TD
    D1["Day 1: Crash"] -->|"open crates<br/>pick up rocks"| D1b["Hammerstone + Stone Blade"]
    D1b -->|"gather sticks + fiber"| D2["Day 2-3: Stone Axe + Pick"]
    D2 -->|"chop trees"| D3["Logs → Saw Horse → Planks"]
    D3 -->|"build bench<br/>place hammerstone"| D5["Day 3-5: First Workshop"]
    D5 -->|"craft rope, tools"| D7["Day 5-7: Charcoal + Fire Economy"]
    D7 -->|"find flint at forest edge"| D10["Day 10: Flint Tools (2-3x durability)"]
    D10 -->|"clay + kiln"| D15["Day 15: Kiln → Brick → Better Buildings"]
    D15 -->|"explore deep forest"| D20["Day 20: Iron Discovery"]
    D20 -->|"smelt + forge"| D30["Day 30: Metal Tools + Forge"]

    classDef early fill:#d97706,stroke:#92400e,color:#fff
    classDef mid fill:#3b82f6,stroke:#1d4ed8,color:#fff
    classDef late fill:#8b5cf6,stroke:#6d28d9,color:#fff
    class D1,D1b,D2 early
    class D3,D5,D7,D10 mid
    class D15,D20,D30 late
```

## Building: Structure

```mermaid
graph TD
    subgraph FLOORS
        RF["Rough Floor — 1 stick"]
        WF["Wood Floor — 2 planks"]
        SF["Stone Floor — 2 rock"]
    end
    subgraph WALLS["Walls (thin, edge-based)"]
        WA["Wattle — 3 sticks + 1 fiber"]
        LW["Low Wall — 2 sticks + 1 fiber"]
        WW["Wood Wall — 2 logs"]
        SW["Stone Wall — 3 rock"]
        GW["Glass — 2 rock"]
        IW["Insulated — 2 clay + 2 planks"]
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
    class RF,WF,SF,WW,SW,GW,IW,BN,BD,CR item
    class WA,LW,TH cheap
```

## Building: Survival & Light

```mermaid
graph TD
    subgraph SURVIVAL
        CF["Campfire — 3 sticks + 1 fiber"]
        CM["Charcoal Mound — 3 logs"]
        SN["Snare — 3 sticks + 1 fiber"]
        CO["Compost — 1 wood"]
        WL["Well — 3 wood + 2 rock + 1 rope"]
    end
    subgraph LIGHTING
        WT["Wall Torch — 1 stick"]
        FL["Floor Lamp — 1 plank"]
        FP["Fireplace — 3 rock"]
    end

    classDef item fill:#3b82f6,stroke:#1d4ed8,color:#fff
    class CF,CM,SN,CO,WL,WT,FL,FP item
```
