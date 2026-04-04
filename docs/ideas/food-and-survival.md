# Food and Survival

Food in Rayworld isn't a resource bar that ticks down. It's a physical process — governed by the thermal sim, the fluid sim, the water system, and the alien ecology. Spoilage is temperature. Cooking is heat transfer. Smoking is fluid dynamics. Irrigation is water flow. Every meal connects to the simulation.

## What Already Exists

- Berry bushes (BT_31) and crops (BT_47) with growing zones
- Hunger need (0.0–1.0) in `PlebNeeds`, decays over time
- Berries (item 0, nutrition 0.20), Raw Meat (item 40, nutrition 0.12), Cooked Meat (item 41, nutrition 0.35)
- Workbench (57), Kiln (58), Saw Horse (61) as crafting stations
- Fireplace (BT_6) produces heat
- Well (BT_59) provides water, clay jugs and buckets carry it
- Thermal sim: per-tile `block_temps` buffer (256×256 f32, actual Celsius)
- Fluid sim: smoke, O₂, CO₂, temperature advection
- Day/night ambient temp (5°C night, 25°C midday)
- Water system planned (water-flow.md): rain, pooling, evaporation, irrigation

## The Core Insight: Food is Temperature

In RimWorld, spoilage is a timer. A freezer is a tagged room. In Rayworld, the thermal sim tracks actual per-tile Celsius. A freezer is a room where the *physics* keeps temperature below zero. Power failure → the room warms at a rate determined by wall material conductivity, insulation thickness, and outdoor ambient. Food spoils because the physics changed.

This transforms food from resource management abstraction to physical engineering.

---

## Alien Crops

This isn't Earth. Nothing edible is familiar. The colonists learn what's safe through trial and error — the frontier doc or a botanist backstory accelerates this.

### Staple: Dustroot

Hardy tuber that grows in poor soil. Low nutrition, bland taste. The "rice" of the frontier — keeps you alive, doesn't make you happy.

- **Growing conditions:** 10–35°C, any soil quality, full sun. Tolerates drought.
- **Yield:** Moderate. Reliable. Grows in 5 days.
- **Raw:** Edible but barely (nutrition 0.15, mood -2 "Ate raw dustroot")
- **Cooked:** Neutral (nutrition 0.25, no mood effect)
- **Eating only dustroot** incurs a stacking mood penalty (see Variety section)

The first crop every colony plants. The last crop anyone wants to eat.

### Toxic if Raw: Bitterbulb

Higher nutrition, but raw bitterbulb causes nausea unless cooked. Heat denatures the toxin.

- **Growing conditions:** 15–30°C, moderate soil, needs water within 4 tiles (irrigation from water-flow.md)
- **Yield:** Lower than dustroot. 7-day growth cycle.
- **Raw:** Toxic. Nausea debuff 6 hours, health damage 0.05. ("Ate raw bitterbulb: -8 mood, vomiting")
- **Cooked (80°C+ for 60s):** Good nutrition (0.35), mild mood buff (+1)
- **Under-cooked:** Toxin remains. Colonist gets sick. The simulation teaches: respect the process.

Requires a kitchen and a cook who knows the temperature. A new cook might under-cook it. The frontier doc knows it needs thorough heating — knowledge-in-people (deeper-systems.md).

### Luxury: Sweetmoss

Grows on damp surfaces near water. Low yield but provides a mood buff.

- **Growing conditions:** 10–25°C, requires moisture (water within 2 tiles or humidity > 0.3 in fluid sim). Shade tolerant.
- **Yield:** Low. Slow growth (10 days). The luxury crop.
- **Raw:** Edible and pleasant (nutrition 0.10, mood +3 "Ate something sweet")
- **Cooked/dried:** Sweetmoss flour → ingredient in bread and desserts
- **Grows well in the greenhouse** (glass + thermal sim trapping heat — emergent-physics.md)

Drives the greenhouse as a mid-game build goal. A colony with sweetmoss has happy colonists.

### Underground: Char-cap

Fungus that grows in the dark, at cool temperatures. Doesn't need sunlight or surface farmland. The cave-farming crop.

- **Growing conditions:** 10–18°C, zero light, any soil. Grows on rock floors underground.
- **Too warm → dies. Too cold → goes dormant.** The thermal sim determines viability.
- **Yield:** Moderate, slow (12-day cycle). Continuous harvest (regrows from mycelium).
- **Raw:** Edible, earthy flavor (nutrition 0.20, no mood effect)
- **Cooked:** Good in soups and stews (nutrition 0.30)
- **The winter crop:** When nothing grows above ground, char-cap in a temperature-controlled underground farm is the only fresh food.

Requires the multi-level system. An early underground farm is a survival investment.

### Tree Crop: Sap-vine

Grows on trees. Harvesting doesn't kill the tree (unlike chopping for wood). Produces calorie-dense sap.

- **Growing conditions:** Grows on living trees. Attach to tree, produces sap over 3 days.
- **Harvest:** Tap sap with bucket or jug. Doesn't destroy the tree. Sustainable.
- **Raw sap:** Sweet, calorie-dense (nutrition 0.15, mood +1)
- **Fermented sap → alcohol** (see Fermentation section). The "grape" of the frontier.
- **Sap syrup:** Boil down at cookfire → concentrated sweetener, ingredient in fine meals.

Creates a reason to preserve trees near the colony instead of clear-cutting for wood.

### Fire-Activated: Bloodgrass

Wild grass that produces edible seeds, but only after being burned. Fire clears the husk; heat activates germination.

- **Growing conditions:** Grows wild in open terrain. Cannot be farmed in growing zones.
- **Harvest:** Burn a field (fire system), wait for regrowth (3 days), harvest seeds from the scorched ground.
- **Seeds:** Nutrition 0.20, mood neutral. Good quantity per burned area.
- **The "controlled burn" crop.** Fire as agriculture — using the fire system for food production.

Creates gameplay around controlled burns. Clear the terrain around the colony (reducing glintcrawler nests per alien-fauna.md), get food as a bonus. But fire near your buildings is risky. Wind direction matters — burn downwind.

---

## Spoilage as Physics

Every food item has a spoilage rate that's a function of temperature at its storage location. Not a fixed timer — a physical process sampled from `block_temps` every tick.

```
spoilage_per_tick = base_rate × temperature_factor(block_temp)

temperature_factor:
  < -10°C  →  0.00   (frozen solid, indefinite storage)
  -10–0°C  →  0.05   (near-frozen, very slow decay)
  0–10°C   →  0.20   (cold storage, slow)
  10–20°C  →  1.00   (room temp, normal rate)
  20–30°C  →  2.50   (warm, accelerated)
  > 30°C   →  5.00   (hot, rapid spoilage)
```

Food items track freshness (1.0 → 0.0). At 0.0: spoiled. Spoiled food is inedible (or edible with food poisoning risk for desperate colonists).

### What This Means in Practice

- Food stored in a properly insulated cold room (0–5°C): lasts weeks
- Food left on a table in a heated room (22°C): lasts a couple of days
- Food forgotten near the kiln (40°C+): spoiled by morning
- Food in the root cellar (underground, 8–12°C): lasts longer than surface storage
- Food in an ice house (see Cold Chain): approaches frozen, lasts months

The player can *watch* food spoil by watching the temperature overlay shift. A power outage that kills the fan cooling your storeroom is a visible emergency — the temperature creeps up degree by degree, and every degree accelerates spoilage.

### Preservation Methods

Preserved food has a much lower base_rate, meaning it resists spoilage even at room temperature:

| Food State | base_rate | Notes |
|-----------|-----------|-------|
| Raw crop | 1.0 | Spoils at normal rate |
| Raw meat | 2.0 | Spoils twice as fast (protein decay) |
| Cooked meal | 0.7 | Slightly more stable than raw |
| Smoked meat | 0.1 | Lasts 10× longer — the preservation breakthrough |
| Dried food | 0.15 | Good for travel rations |
| Fermented | 0.05 | Alcohol and pickled goods barely spoil |
| Frozen | Any × 0.0 | Physics handles this — temperature_factor is 0 |

---

## The Cold Chain

Getting food cold is an engineering problem, not a menu click:

### Root Cellar (Low-tech, free)

Underground level (multi-level.md) is naturally cooler — ground insulates. Costs no power. Limited by depth and soil conductivity. In summer heat waves, even the root cellar warms up — the thermal sim is honest. Best case: 8–12°C year-round.

### Ice House (Seasonal, no power)

Harvest ice from frozen ponds in winter (when thermal sim freezes surface water per water-flow.md). Store underground insulated by straw. The ice melts over months, keeping the room cold. A self-depleting refrigerator restocked each winter. Works only if winters are cold enough to freeze water.

### Fan-Cooled Room (Low power)

A fan (BT_12) blowing outdoor night air into an insulated room. Works only when outdoor temp < indoor temp. The fluid sim handles the air exchange. Useless in summer heat waves. Effective in spring/autumn nights. The cheapest powered option.

### Pipe-Cooled Room (Mid-game, reliable)

Run liquid pipes (BT_49) through a cold source (underground aquifer, shaded water tank) and into the storage room. Cold water absorbs heat from the room. Actual heat-exchange refrigeration using the existing pipe infrastructure. Requires the water system (water-flow.md) and pipe network.

### Evaporative Cooling (Arid biomes)

Wet cloth over a ventilation inlet. Water evaporates, absorbing heat. The fluid sim handles the evaporation physics. Works best in dry climates (low humidity in H₂O dye channel). A desert biome technique. Cheap but requires water.

Each method has different power requirements, seasonal effectiveness, and failure modes. A colony with redundant cold storage survives a power outage. A colony with only fan-cooling loses everything in a summer heat wave.

---

## Cooking as Heat Transfer

Cooking isn't "colonist stands at station, meal appears." Cooking uses fire, heat, and time — physically simulated.

### The Cookfire

A cookfire is the existing fireplace (BT_6) or a new dedicated cooking block. Placing a cooking pot on it (surface item) creates a cooking station. The pot's contents heat based on proximity to fire (sampled from `block_temps`). Different recipes need different temperatures held for different durations.

### Recipes

| Recipe | Ingredients | Temperature | Duration | Result |
|--------|------------|-------------|----------|--------|
| Roast dustroot | 1 dustroot | >150°C | 30s | Nutrition 0.25, mood 0 |
| Bitterbulb stew | 1 bitterbulb + water | 80–100°C | 60s | Nutrition 0.35, mood +1 |
| Char-cap soup | 1 char-cap + water + optional extras | >80°C | 45s | Nutrition 0.30 + bonus per extra |
| Sweetmoss bread | sweetmoss flour + water | >200°C (kiln) | 90s | Nutrition 0.30, mood +4 |
| Sap syrup | 3 sap | >100°C | 120s | Ingredient: sweetener |
| Fine meal | 3+ different ingredients + water + sweetener | >150°C | 120s | Nutrition 0.45, mood +6 |

### Under-cooking and Over-cooking

The thermal sim makes cooking skill-based:

- **Under-cooked bitterbulb:** Pulled off heat before toxin denatured (internal temp < 80°C for required duration). Colonist gets food poisoning. A low-skill cook makes this mistake.
- **Over-cooked / charred:** Left on heat too long at high temp (>250°C for >60s). Nutrition drops, mood penalty. "Ate burnt food: -3"
- **Perfect cook:** Right temperature, right duration. Cooking skill reduces the margin of error — a skilled cook hits the window more reliably.

### Cooking Skill Effect

```
Perfect temperature window = base_window × (1.0 + cooking_skill × 0.15)

Skill 0:  window is tight (±5°C, ±10s) — frequent under/over-cooking
Skill 5:  window is comfortable (±12°C, ±25s) — occasional mistakes
Skill 10: window is generous (±20°C, ±40s) — almost never fails
```

A colony with no skilled cook survives on roast dustroot (simple, hard to mess up). A colony with a great cook produces fine meals from the same ingredients. Cooking skill is the difference between subsistence and thriving.

---

## The Smokehouse — Fluid Sim Showcase

This is the centerpiece. No other game has food smoking driven by actual fluid dynamics.

### How It Works

A smokehouse is:
1. A small enclosed room (3×3 to 5×5 tiles)
2. A fire at the bottom (fireplace BT_6)
3. A vent at the top (partially closed — door or pipe outlet restricting airflow)
4. Meat hung on racks (surface items on poles/hooks)

The fire produces smoke (dye.r channel). The restricted vent traps smoke inside (fluid sim — low-velocity outflow means smoke accumulates). The smoke density at each meat tile determines curing progress. The temperature at each meat tile determines whether you're smoking (good), cooking (bad), or charring (very bad).

### Curing Progress

```
curing_rate = smoke_density × temperature_factor × dt

temperature_factor for smoking:
  < 50°C   → 0.2  (cold smoking — slow, best flavor)
  50–80°C  → 1.0  (hot smoking — ideal range)
  80–120°C → 0.5  (cooking, not smoking — edible but not preserved)
  > 120°C  → 0.0  (charring — food destroyed)
```

Meat reaches "smoked" state when curing_progress hits 1.0. Takes 2–4 hours of gameplay at ideal conditions. Smoked meat has base_rate 0.1 — lasts 10× longer than raw.

### Wind Direction Matters

Wind affects the smokehouse through the fluid sim:

- **Calm wind:** Smoke rises straight up, exits vent slowly. Even distribution. Ideal.
- **Moderate wind into vent:** Pushes smoke back into room. Density increases. Can over-smoke or smother fire (O₂ drops).
- **Wind blowing away from vent:** Draws smoke out too fast. Under-smoking. Meat doesn't cure properly.
- **Wind shift mid-process:** Conditions change. The player who oriented their smokehouse relative to prevailing wind (emergent-physics.md) has fewer problems.

A well-oriented smokehouse with controlled fire and good ventilation produces preserved food that lasts all winter. A badly built one wastes meat and wood. The physics teaches good design.

---

## Hunting Alien Fauna

The creatures from alien-fauna.md aren't just threats — some are food:

### Duskweaver Meat

Edible but unpleasant. Low nutrition, mild toxicity unless thoroughly cooked.

- Raw: nutrition 0.08, mood -5 ("Ate duskweaver")
- Cooked (>150°C, 60s): nutrition 0.18, mood -2 ("Duskweaver stew... better than nothing")
- Starvation food. Nobody eats it unless they have to.
- Abundant — duskweavers are common. A reliable emergency food source.

### Thermogast Steak

Massive calorie payload if you can kill one. Best meat in the game.

- Raw: nutrition 0.30, mood 0
- Cooked: nutrition 0.50, mood +4 ("Thermogast steak — incredible")
- One thermogast feeds the colony for days.
- Plates usable as armor crafting material (late-game connection)
- Killing one is an event, not routine. Requires coordinated combat.

### Glintcrawler

Edible if you remove the venom glands. High-skill preparation.

- Prepared by skilled cook (cooking ≥ 5): nutrition 0.25, mood +1 ("Crunchy. Actually good.")
- Prepared by unskilled cook: 30% chance venom remains → food poisoning, health -0.10
- Risk/reward food. Only worth it with a good cook.

### Ridgeback (New — passive grazer)

Large, slow, herbivorous creature not yet in alien-fauna.md. Grazes in open terrain during the day. The "bison" analog.

- Travels in small herds of 3–6
- Doesn't attack unless cornered. Tough and can trample (damage 15).
- Huntable with ranged weapons. Requires combat skill.
- Substantial meat yield: 8–12 raw meat per kill
- Hide usable for leather (crafting material, future)
- Bones usable for tools (crafting material, future)

The primary hunting target. Seasonal — herds migrate, present for part of the year. Hunting connects to combat (weapons, aim), butchering at workbench (→ meat + hide + bone), and the scent system (blood from butchering attracts predators — emergent-physics.md).

---

## Fermentation and Alcohol

Sap-vine sap or berries in a sealed container at controlled temperature for multiple days. Temperature determines outcome:

| Temperature | Duration | Result |
|------------|----------|--------|
| 5–12°C | 7 days | Weak wine (mood +2, slight intoxication) |
| 15–25°C | 5 days | Strong wine / beer (mood +3, intoxication) |
| 25–35°C | 3 days | Vinegar (preservative, no alcohol — useful as ingredient) |
| > 35°C | Any | Spoiled (wasted) |

**Alcohol has multiple uses:**
- **Stress relief:** Drinking in the saloon (the-human-layer.md) reduces stress. Social drinking with friends = mood buff.
- **Medical antiseptic:** The frontier doc uses alcohol to clean wounds. Better outcomes than dry bandaging.
- **Trade good:** High value-to-weight ratio. Caravans want it.
- **Preservative base:** Alcohol preserves fruit (future recipe: preserved fruit in alcohol).
- **The binge connection:** A stressed colonist (stress > 85) has the "Binge" mental break — raids the liquor supply. Having alcohol enables binge breaks. Not having it removes that specific break but removes the stress relief option too. Tradeoff.

**Distillation (late-game):** A still (new crafting station) converts wine → moonshine. Higher potency, higher value, higher risk of binge. Requires fire + pipe (for cooling the condenser). The pipe system is literally the distillation infrastructure.

---

## Variety and Mood

Eating the same food repeatedly incurs a stacking mood penalty:

```
Same meal 2 days running:   -1 mood
Same meal 4 days:           -3 mood
Same meal 7 days:           -6 mood ("I can't eat another dustroot")
Same meal 14 days:          -10 mood + binge break chance (eats luxury reserves)
```

Eating different foods provides a variety bonus:

```
2 different foods this week:  +1 mood
3 different foods:            +3 mood
4+ different foods:           +5 mood ("Ate well this week")
```

This drives crop diversity, hunting, foraging, and trade. A colony that only farms dustroot survives but is miserable. A colony with dustroot, sweetmoss, smoked ridgeback, and char-cap soup has happy colonists.

---

## Communal Meals

WHERE and HOW you eat matters:

| Context | Mood Effect |
|---------|-------------|
| Eating on the ground | -3 ("Ate without table") |
| Eating at a table, alone | 0 (neutral) |
| Eating at a table with 1–2 others | +2 ("Shared a meal") |
| Eating in the saloon with others | +4 ("Good company over dinner") |
| Eating in the dark | -2 ("Ate in darkness") |

A colonist eating alone in the dark is a red flag — antisocial trait or colony social fabric fraying. Colonists eating together at a well-lit table in the saloon: thriving.

**Sound sim connection:** Meals are sound sources. Clinking, conversation, laughter from a communal meal propagates through the sound sim. Colonists working nearby get a faint mood buff from *hearing* community. An empty, silent dining hall is depressing. A raucous saloon dinner is morale fuel.

---

## Seasonal Pressure

Seasons (gameplay-systems.md) define the food calendar:

**Spring — The Lean Season:**
- Plant crops. Forage sweetmoss from melting stream banks.
- Winter stores running low. The most dangerous time for starvation.
- Ridgeback herds return (migration). First hunting opportunity.
- Mud slows outdoor work (water-flow.md: wet terrain → movement penalty).
- If you didn't preserve enough last autumn: crisis. If you did: comfortable transition.

**Summer — The Abundance:**
- Peak crop growth. Harvest dustroot and bitterbulb.
- Hunt ridgeback herds while they're fat.
- Forage bloodgrass from burned fields.
- Build up stores aggressively. This is the surplus season.
- Spoilage risk highest — temperatures peak, cold storage stressed.

**Autumn — The Preparation:**
- Final harvest. Preserve everything possible (smoke, dry, ferment).
- Harvest sap-vine before frost. Tap the last sap for winter wine.
- Char-cap farm underground producing steadily (temperature-controlled).
- Collect ice if early frost comes (ice house stocking for next year).
- **This is the most important season.** Everything you do in autumn determines whether winter is survivable.

**Winter — The Test:**
- Nothing grows above ground. Eat from stores.
- Char-cap underground farm = only fresh food (if temperature-controlled).
- Hunting harder — fewer creatures, shorter days, cold exposure risk.
- Cold storage is free (outdoor temps below zero) but your *living spaces* need heating.
- A power failure during winter is a double crisis: colonists freeze AND food cold chain fails.

The thermal sim makes this physical. You can watch your cold room temperature on the overlay and know exactly how many days of food you have left based on the spoilage curve.

---

## Water and Food

Water (water-flow.md) connects to food at every level:

- **Cooking** requires water. Stew, bread, soup — all need water as ingredient. No well = no cooked meals beyond basic roasting.
- **Irrigation** from channels or proximity to water boosts crop yield. Bitterbulb *requires* water within 4 tiles. Sweetmoss requires moisture. A colony far from water has limited crop options.
- **Fermentation** needs water for dilution and yeast activation.
- **Cleaning** (future hygiene): a kitchen near a water source produces safer food.
- **Drought** means no irrigation, no stew, no fermentation. Crops wilt. The colony falls back to roast dustroot and dried stores. Water scarcity IS food scarcity.
- **Condensation** (emergent-physics.md): in arid biomes, engineered dew collection might be the only water source, which directly limits what food you can prepare.

---

## Poison and Discovery

Alien food isn't always safe. Colonists learn through trial and error.

**Plant identification:** An unidentified wild plant found during exploration. Eating it raw is a gamble:
- 40% chance: edible, minor nutrition (discover a new food source)
- 30% chance: inedible but harmless (tastes terrible, mood -5, nothing learned)
- 20% chance: mild toxin (nausea 6 hours, health -0.03)
- 10% chance: serious toxin (vomiting, health -0.10, needs medical treatment)

A colonist with the frontier doc or botanist backstory shifts these odds dramatically (70/20/8/2). Knowledge in people (deeper-systems.md).

**Cooking knowledge matters:**
- Bitterbulb toxin denaturation temperature: known to the doc, learned by others through teaching or experience (trial and error with health cost)
- Glintcrawler venom gland removal: cooking skill ≥ 5, or taught by someone who knows
- Fermentation temperature ranges: discovered through experimentation (first batch might become vinegar)
- Smoked meat technique: learned through practice (first attempts may under-cure or char)

**If your cook dies:** The replacement doesn't know the bitterbulb trick, the glintcrawler preparation, or the smokehouse timing. They have to learn — possibly the hard way. This is the knowledge-in-people system applied to food.

---

## The Crash Rations Clock

The starting wreck (the-human-layer.md) contains packaged crash rations:

- ~15 days of food for 3 colonists
- High nutrition (0.40), no mood effect (bland but adequate)
- Don't spoil (sealed packaging)
- Cannot be manufactured — once they're gone, they're gone

The crash rations are the grace period. Day 1–5: eat rations, build shelter. Day 5–10: plant crops, build kitchen. Day 10–15: first harvest should be coming in, start cooking. Day 15: rations run out. If you didn't build a food system, you're foraging berries and eating raw dustroot. If you did, you're eating roast dustroot and bitterbulb stew.

The rations enforce the PHILOSOPHY.md principle: "scarcity as teacher." They give you time to learn, then take away the safety net.

---

## Food Items — Data Model

Extending items.toml with food-specific fields:

```toml
[[item]]
id = 50
name = "Dustroot"
icon = "🥔"
category = "food"
stack_max = 20
nutrition = 0.15
spoil_rate = 1.0        # base spoilage multiplier
raw_mood = -2           # mood effect when eaten raw
cooked_nutrition = 0.25 # nutrition after cooking
cooked_mood = 0         # mood after cooking
cook_temp = 150.0       # minimum °C to cook
cook_time = 30.0        # seconds at temp

[[item]]
id = 51
name = "Bitterbulb"
icon = "🧅"
category = "food"
stack_max = 15
nutrition = 0.10        # raw — low because of toxin nausea
spoil_rate = 1.2
raw_mood = -8           # "Ate raw bitterbulb" + nausea
raw_toxic = true        # triggers nausea/health damage if eaten raw
cooked_nutrition = 0.35
cooked_mood = 1
cook_temp = 80.0        # must reach 80°C
cook_time = 60.0        # for 60 seconds to denature toxin

[[item]]
id = 52
name = "Sweetmoss"
icon = "🌿"
category = "food"
stack_max = 20
nutrition = 0.10
spoil_rate = 1.5        # spoils faster (moist)
raw_mood = 3            # "Ate something sweet"
cooked_nutrition = 0.15
cooked_mood = 4

[[item]]
id = 53
name = "Char-cap"
icon = "🍄"
category = "food"
stack_max = 15
nutrition = 0.20
spoil_rate = 0.8        # fungus keeps okay
raw_mood = 0
cooked_nutrition = 0.30
cooked_mood = 1
cook_temp = 80.0
cook_time = 45.0

[[item]]
id = 54
name = "Sap"
icon = "🍯"
category = "food"
stack_max = 10
nutrition = 0.15
spoil_rate = 1.0
raw_mood = 1
liquid = true           # stored in bucket/jug
```

### Preserved and Cooked Items

```toml
[[item]]
id = 55
name = "Smoked Meat"
icon = "🥓"
category = "food"
stack_max = 10
nutrition = 0.30
spoil_rate = 0.1        # lasts 10× longer
raw_mood = 2            # tasty

[[item]]
id = 56
name = "Dried Dustroot"
icon = "🥔"
category = "food"
stack_max = 20
nutrition = 0.20        # slightly less than cooked
spoil_rate = 0.15       # very stable
raw_mood = -1           # still bland

[[item]]
id = 57
name = "Frontier Wine"
icon = "🍷"
category = "food"
stack_max = 5
nutrition = 0.05        # not much food value
spoil_rate = 0.05       # barely spoils
raw_mood = 3            # social drinking mood
stress_relief = 5.0     # reduces stress when consumed
liquid = true

[[item]]
id = 58
name = "Crash Rations"
icon = "📦"
category = "food"
stack_max = 10
nutrition = 0.40
spoil_rate = 0.0        # sealed, doesn't spoil
raw_mood = 0            # bland but adequate
```

---

## Cooking Recipes — Data Model

Extending recipes.toml:

```toml
# ── Cookfire recipes (requires fire + pot surface item) ──

[[recipe]]
id = 20
name = "Roast Dustroot"
station = "cookfire"
time = 30.0
min_temp = 150.0         # block_temp at station must be ≥ this
inputs = [{item = 50, count = 2}]
output = {item = 50, count = 2, cooked = true}

[[recipe]]
id = 21
name = "Bitterbulb Stew"
station = "cookfire"
time = 60.0
min_temp = 80.0
inputs = [{item = 51, count = 1}, {item = "water", count = 1}]
output = {item = 51, count = 1, cooked = true}

[[recipe]]
id = 22
name = "Char-cap Soup"
station = "cookfire"
time = 45.0
min_temp = 80.0
inputs = [{item = 53, count = 2}, {item = "water", count = 1}]
output_nutrition = 0.30  # base, +0.05 per bonus ingredient
bonus_inputs = [50, 51, 52, 55] # any of these boost quality

[[recipe]]
id = 23
name = "Sweetmoss Bread"
station = "kiln"
time = 90.0
min_temp = 200.0
inputs = [{item = 52, count = 3}, {item = "water", count = 1}]
output = {item_name = "Sweetmoss Bread", nutrition = 0.30, mood = 4}

# ── Preservation recipes (specialized stations) ──

[[recipe]]
id = 25
name = "Dried Dustroot"
station = "drying_rack"   # new surface item: rack near fire
time = 180.0              # slow — 3 minutes of game time
inputs = [{item = 50, count = 4}]
output = {item = 56, count = 4}

[[recipe]]
id = 26
name = "Frontier Wine"
station = "fermenter"     # sealed vessel at controlled temp
time = 300.0              # 5 days at 15-25°C
min_temp = 15.0
max_temp = 25.0
inputs = [{item = 54, count = 3}]  # 3 sap
output = {item = 57, count = 2}
```

Note: Smoked meat doesn't use the recipe system — it's a continuous process driven by the fluid sim (smoke density × temperature × time at the meat's tile). The smokehouse IS the recipe.

---

## Crop Growing Conditions — Data Model

A new `crops.toml` or extension to blocks.toml for crop types:

```toml
[[crop]]
id = "dustroot"
name = "Dustroot"
block_type = 47          # BT_CROP
item_id = 50             # harvests into this item
grow_days = 5
yield = [3, 5]           # min, max items per harvest
temp_min = 10.0          # °C — below this, growth stops
temp_max = 35.0          # above this, plant wilts
water_required = false   # grows without irrigation
light_required = true    # needs sunlight (not underground)
soil_quality_min = 0.2   # grows in poor soil

[[crop]]
id = "bitterbulb"
name = "Bitterbulb"
block_type = 47
item_id = 51
grow_days = 7
yield = [2, 4]
temp_min = 15.0
temp_max = 30.0
water_required = true    # needs water within 4 tiles
water_radius = 4
light_required = true
soil_quality_min = 0.4   # needs decent soil

[[crop]]
id = "sweetmoss"
name = "Sweetmoss"
block_type = 47
item_id = 52
grow_days = 10
yield = [1, 3]
temp_min = 10.0
temp_max = 25.0
water_required = true
water_radius = 2         # needs water very close
light_required = false   # shade tolerant — grows in greenhouse
soil_quality_min = 0.3

[[crop]]
id = "charcap"
name = "Char-cap"
block_type = 47
item_id = 53
grow_days = 12
yield = [2, 4]
temp_min = 10.0
temp_max = 18.0          # narrow range — dies above 18°C
water_required = false
light_required = false   # grows underground in the dark
soil_quality_min = 0.0   # grows on bare rock
regrows = true           # doesn't need replanting
```

Crop growth checks `block_temps` at the tile each tick. If temp is outside range, growth pauses (or plant takes damage if extreme). The thermal sim IS the growing conditions check. A greenhouse with glass roof trapping heat (emergent-physics.md) extends the growing season by keeping soil warm. An underground farm at stable 12°C grows char-cap year-round.

---

## New Blocks and Surface Items

| Block/Item | Type | Purpose |
|-----------|------|---------|
| Cookfire | Block or surface item on fireplace | Cooking station — pot on fire |
| Drying Rack | Surface item | Passive drying near heat source |
| Smoking Rack | Surface item | Holds meat in smokehouse |
| Fermenter | Block (sealed vessel) | Fermentation at controlled temp |
| Still | Block | Distillation (wine → moonshine). Requires fire + pipe cooling |
| Butcher Table | Block | Butcher creatures → meat + hide + bone |
| Dining Table | Block (existing bench?) | Communal eating spot, mood bonus |

---

## Progression Path

The food system has a clear escalation that maps to colony development:

**Days 1–15: Survival (crash rations + foraging)**
- Eat crash rations
- Forage berries from existing berry bushes (BT_31)
- Plant dustroot in growing zone
- Build a well for water

**Days 15–30: Basic Cooking**
- First dustroot harvest → roast on cookfire
- Discover bitterbulb (foraging or growing) → learn to cook it properly
- Build a kitchen area (cookfire + table + water access)
- Colonists eating cooked meals instead of raw food = mood improvement

**Days 30–60: Diversification**
- Multiple crops growing: dustroot, bitterbulb, sweetmoss
- First hunt: ridgeback → butcher → cooked meat
- Build a smokehouse → preserved meat for winter
- Sap-vine tapping → fresh sap → begin fermentation experiments
- Root cellar (first underground room) for cold storage

**Days 60–90: Mastery (first autumn)**
- Smoking, drying, fermenting in full production
- Underground char-cap farm established for winter
- Ice house stocked (if winter was cold enough for ice)
- Pipe-cooled storage room for reliable refrigeration
- Fine meals from multiple ingredients
- Alcohol production for stress relief and trade
- Colony food system can survive winter

**Year 2+: Optimization**
- Greenhouse for year-round sweetmoss
- Distillery for moonshine (high trade value)
- Redundant cold storage (multiple methods)
- Hunting rare creatures (thermogast steak as luxury)
- Exporting preserved food to trade caravans
- Teaching cooking knowledge to backup colonists

---

## Connection to Other Docs

| System | Food Connection |
|--------|----------------|
| **Thermal sim** (`thermal.wgsl`) | Spoilage rate, cooking temperature, crop growing conditions, cold storage, smokehouse temperature |
| **Fluid sim** (`fluid.wgsl`) | Smokehouse smoke density, wind affecting smokehouse, scent from cooking attracting creatures |
| **Water system** (`water-flow.md`) | Irrigation, cooking ingredient, fermentation, crop water requirements, drought → food crisis |
| **Alien fauna** (`alien-fauna.md`) | Hunting for meat, creatures attracted to food stores, ridgeback as primary game animal |
| **Needs system** (`needs.rs`) | Hunger need drives eating, mood effects from food quality/variety/context |
| **Crafting** (`crafting.md`) | Recipes extend existing system, new stations (cookfire, smokehouse, fermenter, still) |
| **Knowledge** (`deeper-systems.md`) | Cooking skill, plant identification, recipe knowledge lives in people |
| **Psychology** (`deeper-systems.md`) | Variety bonus, communal meals, binge mental break + alcohol |
| **Seasons** (`gameplay-systems.md`) | Growing calendar, preservation urgency, winter as survival test |
| **The human layer** (`the-human-layer.md`) | Crash rations clock, saloon meals as social event, moral choices around food scarcity |
| **Emergent physics** (`emergent-physics.md`) | Greenhouse, condensation for water, wind orientation for smokehouse, scent system |
| **Multi-level** (`multi-level.md`) | Root cellar, underground char-cap farm, ice house storage |

---

## What Makes This Different From RimWorld

| Aspect | RimWorld | Rayworld |
|--------|----------|----------|
| Spoilage | Fixed timer per food type | Physics — temperature at storage tile × spoilage rate |
| Freezer | Room tagged "cold" | Room where thermal sim reads < 0°C. Power failure = warming curve |
| Cooking | Colonist stands at stove, meal appears | Heat transfer: fire → pot → food. Under/over-cooking possible |
| Smoking | Not in base game | Fluid sim: smoke density × temperature × time. Wind matters |
| Crop growth | Tile fertility + growing season | Thermal sim temp range + water proximity + light + soil quality |
| Food variety | Minor mood effects | Stacking mood penalties/bonuses that drive real behavior |
| Preservation | Freezer solves everything | Multiple methods, each with physics-based tradeoffs |
| Alcohol | Crafted at bench, consumed | Fermentation at controlled temperature. Too hot = vinegar. Distillery uses pipe system |
