# DN-021: UI Philosophy

**Status:** Proposed
**Depends on:** DN-018 (equipment), DN-019 (knowledge and crafting)
**Affects:** ui.rs, raytrace.wgsl, simulation.rs, pleb.rs

## Summary

The world is the interface. Instead of panels that TELL the player what's happening, make the simulation SHOW them. The raytracer, the fluid sim, the thermal sim, and the sound sim already produce the most information-dense rendering in any colony sim — the UI philosophy is to lean into that, minimizing screen chrome and letting the world communicate state visually. Where chrome is necessary, it should feel like frontier communication: physical, spatial, and occasionally unreliable.

## The Problem with Standard Colony Sim UI

RimWorld, ONI, Dwarf Fortress: a viewport with chrome bolted onto the edges. Panels for information. Bars for resources. Buttons for tools. The viewport is passive — you look through it and click things. The UI exists in a separate visual layer overlaid on the game.

The current Rayworld UI (11,860 lines of egui) follows this pattern: colonist portraits top-center, build categories bottom-left, resource bar top-left, overlays, minimap. It works. But it doesn't leverage the thing that makes this game unique: **a simulation that's always running, always visible, always communicating.**

## Design Principle: Minimize Chrome, Maximize World

Every piece of information should first be asked: "Can the world show this instead of a panel?" If the answer is yes — even partially — the world shows it and the panel becomes a precision drill-down, not the primary reading.

---

## Part 1: Always-Visible State Through the Raytracer

### Thermal State Without an Overlay

Right now: toggle a temperature overlay to see heat. Proposal: temperature is always subtly visible through the raytracer's light color.

Cold areas have a faint blue tint on surfaces. Warm areas have a faint amber warmth. Hot areas shimmer. The player never needs to open an overlay to see "the kiln room is warm" — the light tells them.

Implementation in `raytrace.wgsl` — a subtle color shift applied to the final pixel based on the temperature at that tile:

```wgsl
// After final_color is computed, apply subtle thermal tinting
let tile_temp = block_temps[tile_idx];
let temp_norm = clamp((tile_temp - 10.0) / 30.0, -1.0, 1.0); // -1 cold, +1 hot
let cold_tint = vec3<f32>(0.92, 0.95, 1.0);  // very subtle blue
let warm_tint = vec3<f32>(1.0, 0.97, 0.92);  // very subtle amber
let tint = mix(cold_tint, warm_tint, temp_norm * 0.5 + 0.5);
final_color = final_color * mix(vec3(1.0), tint, 0.08); // 8% blend — barely visible
```

At 8% blend the tint is subconscious — the player FEELS the temperature difference between rooms without consciously seeing a blue overlay. The precision temperature overlay (existing toggle) remains for when you need exact numbers. But the world's default state already whispers "warm" and "cold."

### Air Quality Through Visual Haze

The fluid sim already renders smoke. Extend this to air quality in general:

- **Low O₂ areas:** Subtle visual murkiness — a faint darkening that says "this room has thin air." Not enough to obscure detail, enough that your eye reads "something's off."
- **High CO₂ areas:** Very faint greenish-amber tint. A sealed room with a campfire and no ventilation slowly develops a visible haze.
- **Fresh air:** Crystal clear rendering. The contrast between well-ventilated and stale rooms is visible without any overlay.

Implementation: the fluid sim already tracks gas concentrations per tile. The raytracer samples these and applies a per-pixel tint/opacity modifier. The modifier is proportional to concentration and subtle enough to be subliminal at moderate levels, obvious only when conditions are dangerous.

### Sound Through Visual Ripple

The sound sim propagates waves. When a loud sound occurs (explosion, shout, bell), the raytracer could show a subtle concentric ripple emanating from the source — like heat shimmer but circular. Visible for only a fraction of a second. Not cartoon. More like a pressure wave distortion in the air.

This makes the sound sim visible at critical moments. You SEE where the gunshot came from before you hear it (speed of light > speed of sound, even in game terms). You SEE the shockwave of an explosion ripple across the colony. It's physically correct and visually dramatic.

### Light as Information

The raytracer's per-pixel lighting already communicates enormous amounts of state:

- **Time of day** from the sky color. Dawn is pink-gold, noon is white-blue, dusk is amber-red, night is deep blue-black. The player knows the time by looking at the light.
- **Weather** from the ambient light. Overcast = flat, diffuse lighting. Clear = sharp shadows. Storm = dramatic, flickering.
- **Season** from the color temperature. Spring = cool greens. Summer = warm yellows. Autumn = amber. Winter = blue-white.
- **Power state** from artificial light. A building with power has warm artificial lighting. A building that lost power goes dark. You see the power outage before any notification tells you.
- **Fire** from its light. A fire in a distant building throws flickering amber light visible from across the map — an instant visual alarm.

Every light source in the game is a signal. The player who learns to read light reads the colony's state at a glance.

---

## Part 2: Colonist State Through Body Language

Instead of reading mood numbers on a panel, colonists SHOW their state:

### Movement and Posture

| State | Visual Cue |
|-------|-----------|
| Happy (mood > 70) | Walks upright, slightly faster, head turns toward friends when passing |
| Content (mood 40–70) | Normal walk, neutral posture |
| Stressed (mood 20–40) | Slight hunch, slower walk, avoids eye contact (head turns away from others) |
| Crisis (mood < 20) | Pronounced hunch, very slow, may stop and stand still facing a wall |
| Injured | Limp on the affected side, hand on wound location |
| Hungry | One hand on stomach periodically during idle |
| Exhausted | Head droops, shuffling gait, yawning animation |
| Cold | Arms wrapped around body, breath fog more visible |
| Hot | Slower movement, wiping motion |

These are subtle sprite/animation changes, not exaggerated cartoon emotes. The player who watches their colony closely reads mood from behavior. The mood panel still exists for precise numbers, but the world communicates first.

### Skill Confidence

A colonist's knowledge level (DN-019) is visible in how they work:

- **Familiar** at the smelter: Pauses before each step. Looks at the ore, then back at the furnace. Hesitant movements. Visible uncertainty. The player watches and thinks "she doesn't really know what she's doing" — then checks the panel and confirms: Familiar.
- **Competent:** Smooth, steady workflow. No pauses. Confident but not flashy.
- **Expert:** Casual speed. Doesn't look at what they're doing — their hands know the work. Occasionally glances away (at other colonists, at the horizon). Relaxed mastery.
- **Master:** Same as Expert but with occasional distinctive flourishes — a unique way of holding the hammer, a signature gesture. Their work has style.

### Social Reading

- Two colonists with high friendship walk close together, turn toward each other.
- Two colonists with rivalry keep distance, turn away.
- A teaching pair (Expert + Familiar) shows the Expert gesturing and the Familiar watching intently.
- An argument shows both facing each other, bodies tense, occasional arm gestures.
- A leader speaking shows others turning toward them, bodies oriented to the speaker.

The colony's social topology is visible from the way people move through space relative to each other. You can read friendship clusters, social isolation, teaching relationships, and conflicts without opening any panel.

---

## Part 3: Diegetic Interface Elements

Some UI elements live IN the world as physical objects, not ON the screen as floating panels.

### The Notice Board

A buildable block (wooden post with a board). Clicking it opens the colony's work queue — active tasks, priorities, pending orders. Colonists walk to the notice board at shift changes to check assignments (visible activity). Moving the notice board changes where colonists gather.

- **Gameplay function:** Work priority management, task queue viewer.
- **Diegetic function:** Physical location where information lives. A colony with a centrally-placed notice board has efficient task updates. One tucked in a corner means colonists waste time walking to it.
- **Visual:** Small but visible in the world. Paper scraps tacked to a wooden board. More active orders = more cluttered board.
- **If destroyed:** Tasks still queue (the system works), but colonists can't check priorities efficiently — they default to nearest task rather than highest priority. Rebuild the board to restore prioritization.

### The Cartography Table

A buildable block where the world map lives. Clicking it opens the world map (expedition planning, world intelligence, outpost management from world-and-seasons.md). A colonist assigned as cartographer updates the map based on expedition reports.

- **Gameplay function:** World map interface, expedition dispatch.
- **Diegetic function:** The map is a physical artifact in the colony. If the cartography table burns, you don't lose the world map data (colonists remember), but you lose the ability to plan expeditions until it's rebuilt.
- **Visual:** A large flat table with a visible map surface. Colonists lean over it when planning.

### The Library as Knowledge Browser

Instead of a floating knowledge panel, the library bookshelves serve as the knowledge UI. Click a shelf → see what lore items are stored, what domains they cover, what level of understanding they convey.

- **Visual feedback:** More books = fuller shelves. A burned library has visible empty shelves — the devastation is physical. A well-stocked library looks dense, established, permanent.
- **Colonist interaction:** You see colonists studying at the library — sitting at a desk, reading. The teaching-from-books activity is visible. A colonist who needs knowledge walks to the library autonomously.
- **Trade view:** Opening the library during trade shows which lore items the trader wants and what you'd lose by selling.

### The Radio as Event Feed

Instead of event notifications popping up as floating text, the radio block crackles and speaks:

- **Sound sim integration:** Radio broadcasts are sound sources at the radio's position. Walk near it and you hear the current broadcast as ambient audio.
- **Visual:** A small bubble appears above the radio with the broadcast topic (using the chat bubble system from social-knowledge.md). Click to expand.
- **Without a radio:** No external event notifications. No trade caravan warnings. No weather alerts (unless a watchtower exists). The colony is deaf to the outside world until they build or salvage a radio.
- **Multiple radios:** Place radios in different buildings. Each one functions as a local speaker — colonists near a radio hear broadcasts. One in the saloon keeps everyone informed during evening social time. One in the workshop keeps the workers updated.

### Crafting Station as Recipe Browser

No global crafting menu. Click on the workbench → see what recipes are available based on who's nearby, what they know, and what materials are stocked. The recipe list is contextual:

- Workbench near Kai (Expert metallurgy) shows metal recipes.
- Same workbench with Jeb nearby (Familiar cooking) shows basic cooking recipes at reduced quality.
- No colonist near the workbench → "No one available to work here."
- Missing materials are grayed out with a note: "Need 3 iron ore (have 0)."

The UI is: click station → see what THIS person can make HERE with WHAT'S AVAILABLE. Not an abstract crafting tree. The three-lock system (DN-019) is visible in the recipe list — knowledge lock, material lock, infrastructure lock, all shown contextually.

---

## Part 4: The Chrome That Remains

Some information needs to be on screen. But it can be distinctive.

### Colonist Bar as Silhouettes

Replace portrait icons at the top with colonist **silhouettes** — tiny side-profile shapes showing posture, equipment, and state:

- **Equipment visible:** A colonist with a pack shows a bulge on their back. One with a rifle has a long slung shape. Belt items are tiny bumps at the waist.
- **Posture reflects mood:** Happy = upright. Stressed = hunched. Sleeping = horizontal line. In combat = crouched.
- **Health indicators:** An injured colonist's silhouette shows a small red indicator at the wound location. A sick colonist has a faint green tint.
- **Activity indicator:** Working colonists have a tiny tool icon near their hands. Walking ones have subtle motion lines. Idle ones stand still.

The silhouettes update in real-time. You read "who's working, who's sleeping, who's in trouble" from tiny shape changes. Clicking a silhouette still opens the detailed panel. But the bar itself communicates MORE than static portraits.

**Alternative explored:** Circular portraits (current RimWorld style) vs. silhouettes. Portraits show the face — identity-focused. Silhouettes show the whole body — state-focused. For a game where what someone IS DOING matters more than what they LOOK LIKE, silhouettes communicate more per pixel. But this is a style decision that could go either way.

### Resource Bar as Physical Piles

Instead of "🪵 47" in text, show resources as tiny visual stacks:

- **Wood:** A small pile icon that grows taller with more wood. Under 10 = a few sticks. 50+ = a proper lumber stack. 100+ = a large pile.
- **Stone:** A rock pile that accumulates. Different stone types have different colors.
- **Food:** Show the actual items as miniature icons — berries, dried meat strips, bread loaves. The variety is visible: a colony eating only dustroot shows a monotone food bar. One with diverse food shows a colorful mix.
- **Iron:** Metal ingots stacked. Empty = concerning. Full = satisfying.

Resources feel physical, not numerical. The exact number appears on hover. At a glance, the player reads "healthy surplus" or "running low" from pile size, not from parsing digits.

The bar lives at the top-left where it is now. But it communicates through visual weight rather than text. A colony with abundant resources has a visually dense resource bar. A struggling colony has a sparse, thin bar. The bar is a barometer of prosperity.

### Contextual Radial Menus

When selecting a colonist or building, instead of (or in addition to) a side panel, a small **radial menu** blooms around the selection:

**Colonist radial (first interaction level):**
- North: Prioritize / Work orders
- East: Equipment / Loadout
- South: Draft for combat
- West: Social / Knowledge

Each petal is an icon, not text. Clicking a petal either executes (draft) or opens the relevant detail panel (equipment opens the Diablo-grid view). The radial is fast — one click to select, one click to act. No panel hunting.

**Building radial (first interaction level):**
- North: Info / Status
- East: Set production (if crafting station)
- South: Deconstruct
- West: Modify (rotate, upgrade, change settings)

The radial disappears when you click elsewhere. It's ephemeral — appears on demand, vanishes when done. No persistent panel clutter.

### Edge-of-Screen Environmental Indicators

The viewport edges subtly communicate what's beyond the visible area:

- **Amber glow at an edge:** Fire burning off-screen in that direction. Intensity indicates distance/severity.
- **Blue tint at an edge:** Cold weather approaching from that direction (world map weather system).
- **Small creature silhouettes at the edge:** Duskweavers or other threats just outside the view. They skitter along the border.
- **Tiny sound ripple indicators:** Show where off-screen sounds originate. A gunshot to the northeast shows a ripple emanating from the northeast viewport edge.
- **Mist creeping in from an edge:** Fog approaching from that direction.
- **Warm glow at an edge:** Colonists gathered around a fire just outside view (friendly presence).

The edges are a peripheral awareness system. You don't need a minimap if the viewport edges tell you what's around you. The existing minimap becomes an optional precision tool rather than the primary spatial awareness mechanism.

### The Clock as Ambient Light

Instead of a numerical clock dominating the UI, the sun/moon position communicates time:

- **A thin arc** at the very top edge of the screen showing the sun's position (golden dot on the arc). At dawn it's left, noon it's center, dusk it's right. At night the arc shows a pale moon dot.
- **No numbers unless hovered.** The arc is a visual whisper, not a numerical display.
- **The sky itself is the clock.** Dawn = pink. Morning = warm gold. Noon = bright white. Afternoon = warm. Dusk = amber-red. Night = deep blue. The player learns time from color unconsciously.

Season is shown as a small marker on the arc: a colored segment indicating spring (green), summer (gold), autumn (amber), winter (blue-white). The player always knows where in the year they are without a dedicated panel.

---

## Part 5: Equipment and Knowledge as Spatial UI

### Equipment on Inspection

When a colonist is selected, their equipment appears as a small **paper-doll overlay** near them in the world — not in a separate panel:

- Belt slots shown as small rectangles at waist level
- Vest slots on the chest area
- Pack as a shape on the back
- Active item (in-hand) highlighted

This is the first-level view. You see the colonist's loadout spatially, in context, on their body. Clicking any slot opens the detailed Diablo-grid view (DN-018) as a proper panel. The first interaction is spatial and immediate; the deep dive is traditional.

### Knowledge as Observable Behavior

Rather than a knowledge bar on a colonist's stat page, knowledge levels are first communicated through observable behavior (Part 2 above) and through the social system:

- You see Kai teaching Jeb at the smelter. Teaching implies: she's Expert+, he's learning.
- The notification "Jeb is now Familiar with metallurgy" arrives through the log.
- The library shows what's documented.
- Clicking a colonist's detailed panel shows the knowledge bars for precision.

The gradient is: **observe → notice → inspect.** The world shows behavior. Notifications confirm transitions. Panels provide numbers. Three layers, from ambient to precise.

---

## Part 6: Notifications as Frontier Communication

Event notifications should arrive through channels that feel physical, not through floating UI popups.

### The Shout

Urgent information: "Enemies spotted!" is a colonist shouting — visible as a large speech bubble with an exclamation mark, audible through the sound sim. Colonists in earshot react (turn heads, change posture). The shout propagates through the world like a real sound:

- Colonists indoors behind walls may not hear the shout.
- The player hears it (as a spatial sound cue) and sees the bubble.
- The shout's location is information: if it came from the south perimeter, that's where the threat is.

### The Colony Log

A persistent scrollable journal accessible from a UI button (or from clicking the writing desk). Records everything: events, discoveries, conversations, trades, births, deaths. Dated by game day.

The log doesn't interrupt — the player goes to it when they want history. It's the authoritative record. The log entry for a discovery includes who discovered it, when, and how: "Day 37, autumn: Kai discovered iron ore near the eastern ridge while prospecting."

### The Bell Tower

A buildable structure. When built, the colony has an alert system:

- **Threat detected:** Bell rings (sound sim propagation). Bell icon pulses at the top of screen. Click → see the alert.
- **Weather incoming:** Single chime. Subtle.
- **Caravan arriving:** Double chime. Trade opportunity.
- **Colonist in crisis:** Rapid ringing. Urgent.

**Without a bell tower:** No structured alerts. Shouts still happen (organic, short-range). But there's no colony-wide notification system. The player relies on watching the world directly. Building a bell tower is an investment in information infrastructure — like building the library is an investment in knowledge infrastructure.

The bell tower teaches: communication requires physical systems. On a frontier planet, nobody alerts you unless someone builds the mechanism to do it.

### Tiered Notification Design

```
Layer 1 — World-first (always on, no chrome):
  Visual: Light changes, smoke, creature movement, body language
  Sound: Environmental audio, creature calls, weather
  → Player reads the world

Layer 2 — Diegetic (requires built infrastructure):
  Bell tower alerts, radio broadcasts, notice board updates
  → Player built the information infrastructure

Layer 3 — Organic social (automatic, no infrastructure):
  Shouts, chat bubbles, colonist reactions
  → Colonists communicate naturally

Layer 4 — UI chrome (always available, minimal):
  Colony log, minimap, stat panels on click
  → Traditional UI as precision tool, not primary channel
```

The game starts at Layer 1 + 3 only. Layer 2 is built. Layer 4 is always there but secondary. The player who engages with Layers 1–3 has a richer experience than one who relies on Layer 4 alone. But Layer 4 is never withheld — it's safety net, not barrier.

---

## Part 7: Seasonal UI Shifts

The UI chrome itself subtly shifts with seasons, reinforcing the mood the raytracer establishes in the world:

**Spring:** Clean, fresh. Slightly warmer background tint on panels. Borders feel lighter. The UI breathes.

**Summer:** Warm tint. Panels feel open, bright, crisp. Sharp edges. The confidence of abundance.

**Autumn:** Amber tinge. Something about the UI feels heavier, denser. Borders slightly thicker. The weight of preparation.

**Winter:** Cooler tint on panel backgrounds. A very faint condensation texture on the edges of the colonist bar — as if the panel itself is cold. Borders feel tighter, more enclosed.

This is purely cosmetic. No functionality changes. But it reinforces the seasonal arc at a subliminal level. The player who plays through a full year unconsciously associates the UI tint with the season's pressure. When they see the amber tinge returning, they feel the urgency of autumn preservation before they consciously register the season change.

### Weather Effects on UI

During a blizzard, subtle screen-edge frost creeps inward. Not enough to obscure — a thin decorative border of ice crystals on the viewport edge. During rain, very faint droplet traces on the "camera lens" (the viewport). During a heat wave, a barely-perceptible shimmer at the screen edges.

These effects are the thinnest cosmetic layer. They can be toggled off for players who prefer clean UI. But when on, they make the viewport feel like a window into the world rather than a detached camera floating above it.

---

## Part 8: What Changes in ui.rs

### Current Structure (what stays)

The existing egui layout isn't thrown away. The build bar, the colonist bar, the resource bar, the overlays — these all remain as the Layer 4 safety net. What changes is their visual treatment and their role:

| Current Element | Stays? | Change |
|----------------|--------|--------|
| `draw_resource_bar` | Yes | Visual pile icons instead of text counts. Numbers on hover. |
| `draw_colonist_bar` | Yes | Silhouettes instead of (or alongside) portraits. Real-time posture/equipment. |
| `draw_build_bar` | Yes | No change needed. Building categories are functional, not informational. |
| `draw_menu_bar` | Yes | Simplified. Season/time indicator as arc, not text clock. |
| `draw_selection_info` | Yes | Supplement with radial menu for first interaction. Panel for deep dive. |
| `draw_minimap` | Yes | Made optional. Edge-of-screen indicators reduce reliance. |
| `draw_notifications` | Rewrite | Tiered system: world-first → diegetic → organic → chrome. |
| `draw_overlays_and_popups` | Yes | Overlays become precision tools rather than primary reading. |
| `draw_game_log` | Yes | Enhanced: the colony journal. Becomes more central. |
| `draw_world_labels` | Yes | Enhanced: chat bubbles from social-knowledge.md integrated. |
| `draw_conditions_bar` | Reduce | Conditions readable from world state (thermal tinting, body language). |
| `draw_hover_info` | Yes | Enhanced: richer contextual info on hover. |

### New Elements to Add

| New Element | Purpose | Priority |
|-------------|---------|----------|
| Thermal tinting in raytrace.wgsl | Always-visible temperature | High — small code change, big impact |
| Chat bubble rendering | Social knowledge visualization | High — core to DN-019 |
| Body language animation | Mood/skill readable from sprites | Medium — requires DN-009 sprite work |
| Radial context menus | Fast first-interaction on selection | Medium — UX improvement |
| Edge-of-screen indicators | Peripheral environmental awareness | Medium — atmospheric, reduces minimap reliance |
| Resource pile visualization | Physical resource reading | Low — cosmetic upgrade to existing bar |
| Colonist silhouettes | State-at-a-glance in colonist bar | Low — alternative to portraits |
| Notice board / bell / radio blocks | Diegetic information infrastructure | Medium — new block types, moderate integration |
| Sun/moon arc | Ambient time display | Low — replaces text clock |
| Seasonal UI tinting | Subliminal seasonal reinforcement | Low — pure cosmetic |
| Crafting station context UI | Knowledge-gated recipe browsing | High — core to DN-019 crafting flow |

---

## Part 9: Connection to Other Systems

| System | UI Connection |
|--------|--------------|
| **Raytracer** (raytrace.wgsl) | Thermal tinting, air quality haze, sound ripples, seasonal palette, fire light as alarm. The raytracer IS the primary UI. |
| **Sound sim** (sound.wgsl) | Shouts propagate physically. Bell tower alerts have range. Radio broadcasts are positional sound. |
| **Fluid sim** (fluid.wgsl) | Smoke visibility = air quality indicator. Mist and dust communicate weather. Breath fog communicates cold. |
| **Thermal sim** (thermal.wgsl) | Always-visible temperature tinting. Frost patterns on glass. Heat shimmer. |
| **Sprites** (DN-009) | Body language, posture, work confidence, social orientation all require sprite expressiveness. |
| **Equipment** (DN-018) | Paper-doll overlay. Equipment visible on silhouettes. Radial menu for quick loadout access. |
| **Knowledge** (DN-019) | Library as knowledge browser. Crafting station context. Skill confidence in animation. Teaching visible as activity. |
| **Social** (social-knowledge.md) | Chat bubbles. Conversation visualization. Personality readable from social behavior. |
| **Seasons** (world-and-seasons.md) | Seasonal UI tinting. Weather effects on viewport edges. Sun arc position. |
| **Building** | Diegetic blocks: notice board, bell tower, cartography table, radio. Information infrastructure as buildable objects. |

---

## Open Questions

1. **How much diegetic is too much?** The notice board, bell tower, cartography table, radio, and library are all diegetic. If every interface element requires a building, early game (no buildings) has no UI. Recommendation: Layer 4 (traditional chrome) is ALWAYS available. Diegetic elements enhance and replace gradually as you build them. A colony with no bell tower still gets a small notification badge — it just doesn't ring through the sound sim.

2. **Silhouettes vs. portraits?** Both communicate different things. Silhouettes show state, portraits show identity. Could do both: portrait underneath, silhouette overlay that updates. Or: let the player choose in settings. Recommendation: start with silhouettes. Identity comes from learning the colonist's colors and posture, not from a static face icon.

3. **Edge indicators vs. minimap?** Edge indicators are atmospheric but imprecise. The minimap is precise but generic. Both could coexist — the minimap shows exact positions, edge indicators show environmental context (fire, cold, creatures). Recommendation: keep both. The minimap becomes a toggleable precision tool. Edge indicators are always-on ambient awareness.

4. **How readable is thermal tinting at 8% blend?** Needs playtesting. If it's too subtle, it's invisible. If it's too strong, it's annoying. The blend factor should be a settings slider: "World ambient indicators: Off / Subtle / Visible." Default to Subtle. Players who want clean rendering can turn it off. Players who want maximum world-readability turn it up.

5. **Radial menus: mouse or keyboard?** Radial menus work beautifully with mouse (bloom from click point). They also work with keyboard shortcuts (1/2/3/4 for each petal). Both should be supported. The radial is the VISUAL representation; keyboard shortcuts are the fast-path.

6. **Should the build bar change?** The current build bar (bottom-left, categories + items) is functional and clear. It's the one piece of traditional UI that doesn't need reinvention. But it could optionally be replaced by a radial on right-click: right-click empty ground → build radial with categories. Recommendation: keep the build bar. Add right-click radial as a power-user shortcut, not a replacement.

---

## Summary

The UI philosophy is layered information delivery: **world first, infrastructure second, organic social third, chrome last.**

**The world is the primary interface.** Thermal tinting, air quality haze, light-as-information, body language, work confidence animations, and equipment visibility on sprites all communicate state through the simulation itself. The player who watches the world closely knows what's happening without opening any panel. The raytracer's per-pixel physics make this possible in a way no tile-based renderer can match.

**Diegetic elements make information physical.** The notice board, bell tower, cartography table, radio, and library are all buildable blocks that serve as both gameplay objects and interface access points. Information infrastructure must be built, can be destroyed, and has physical presence. The colony's ability to communicate with itself and the outside world depends on what it has constructed. This is PHILOSOPHY.md's permanence principle applied to UI.

**Organic social communication happens automatically.** Chat bubbles, shouts, teaching animations, and social body language are always present. The social knowledge system (DN-019) reveals itself through visible colonist interaction, not through panels. The player sees the knowledge network in action.

**Traditional chrome is the safety net.** Every piece of information is also accessible through standard panels, bars, and menus. Nothing is hidden behind diegetic requirements — if the bell tower hasn't been built, alerts still show (just less prominently). The chrome layer ensures the game is playable for players who prefer traditional UI. But it's designed to feel secondary — the player who reads the world has a richer, more immersive experience than one who reads the panels.

The result: a colony sim where the UI is invisible because the world IS the UI. The simulation communicates. The buildings communicate. The people communicate. The panels are there for precision, not for primary awareness. The game feels like a world you're looking into, not a spreadsheet you're operating.
