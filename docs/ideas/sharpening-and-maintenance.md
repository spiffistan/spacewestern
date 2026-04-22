# Sharpening and Tool Maintenance

Currently: tools auto-sharpen in the field (1.5s pause, restores to 60%). No infrastructure needed. This is the primitive baseline.

## Progression Tiers

### Tier 0: Field Sharpen (current)
- Pleb pauses briefly, sharpens with whatever's at hand
- Restores to 60% of max durability
- No cost, no tools needed
- Feels scrappy — appropriate for crash survivors

### Tier 1: Whetstone at Workbench
- Craft a whetstone (2 rock at workbench)
- Place near workbench or carry in inventory
- Restores to 80% instead of 60%
- Faster sharpen (0.8s vs 1.5s)
- Pleb walks to workbench when tool gets worn instead of field-sharpening

### Tier 2: Grinding Wheel
- Craft a grinding wheel (stone + wood + rope)
- Permanent workshop fixture
- Restores to 95%
- Also works on metal tools (future)
- Could use water for cooling (wet grinding — tie into fluid system)

## Sharpness vs Durability (future split?)

Right now sharpness IS durability. A dull axe and a cracked handle are the same bar. Splitting them:

**Sharpness** (head component):
- Affects cutting speed (dull axe = slower chop)
- Restored by sharpening (cheap, fast)
- Degrades with every use

**Durability** (handle + binding):
- Affects when the tool breaks entirely
- Restored by repair (costs materials)
- Degrades on impact/stress

This would mean a well-maintained axe that's regularly sharpened lasts much longer than one that's never maintained — the sharpening prevents the head from chipping under stress.

Not worth implementing yet. The single durability bar creates the right gameplay pressure. Split only if players report confusion about "why does sharpening not fully fix my tool?"

## Repair at Workbench

Broken tools (durability 0) currently just sit on the ground. Future:
- Idle pleb with crafting priority picks up broken tool
- Walks to workbench
- Repairs: costs 1 unit of the failed component's material (per DN-032)
- Repaired tool has 60% durability
- Auto-triggered by minimum stock system (keep N tools in stock)
