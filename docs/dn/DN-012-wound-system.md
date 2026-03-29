# DN-012: Wound System — Anatomical Hit Locations, Bleeding, Infection, Treatment

## Status: Proposed

## Problem

Combat currently does flat HP damage: `health -= 0.2`. A bullet to the head and a bullet to the foot deal identical damage with identical consequences. There's no bleeding, no fractures, no infection, no medical treatment beyond "health regenerates." Combat has no lasting consequences.

## Solution

Replace flat HP with an anatomical wound system. Each bullet hit creates a specific wound on a specific body part. Wounds bleed, hurt, get infected, require treatment, heal over time, and sometimes leave permanent marks. The colony needs a medical system — bandages, a doc, a clinic — or wounds become death sentences.

## Anatomy — Hierarchical Body Tree

### Architecture: Data-Driven Body Definitions

Anatomy is NOT a flat enum. It's a tree defined in data (`bodies.toml`), where each node is a body part that can contain sub-parts. The collision system resolves the **coarse region** (head/torso/arm/leg) from hit position, then a weighted random roll within that region selects the **specific part** — down to individual fingers, organs, or eyes.

This means anatomy is infinitely expandable by editing a TOML file. No code changes to add new body parts.

### Body Part Node

```rust
/// A single node in the anatomy tree. Defined in bodies.toml, loaded at startup.
pub struct BodyPartDef {
    pub id: &'static str,           // "left_eye", "right_kidney", "left_index_finger"
    pub name: &'static str,         // "Left Eye", "Right Kidney"
    pub parent: Option<&'static str>, // "head", "torso", "left_hand"
    pub region: HitRegion,          // coarse collision region (Head, Torso, LeftArm, etc.)
    pub hit_weight: f32,            // relative probability within region (bigger = more likely hit)
    pub max_hp: f32,                // structural integrity (0 = destroyed)
    pub pain_factor: f32,           // how much pain wounds here cause (eyes > torso > fingers)
    pub bleed_factor: f32,          // how much wounds here bleed (arteries > extremities)
    pub vital: bool,                // destruction = death (brain, heart, both lungs)
    pub locomotion: f32,            // contribution to movement (legs, feet, knees)
    pub manipulation: f32,          // contribution to hands/arms work (fingers, hands, elbows)
    pub cognition: f32,             // contribution to thinking (brain, eyes)
    pub breathing: f32,             // contribution to respiration (lungs, trachea)
    pub armor: f32,                 // natural damage reduction (skull > skin)
    pub can_amputate: bool,         // can be surgically removed
    pub children: Vec<&'static str>, // sub-parts (resolved at load time)
}

/// Coarse hit regions — determined by collision geometry, not data
#[derive(Clone, Copy)]
pub enum HitRegion {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}
```

### Humanoid Body Tree

```
Body
├── Head                          [region: Head, hit_weight: 1.0]
│   ├── Skull                     [armor: 0.4, vital: false]
│   │   └── Brain                 [vital: true, cognition: 1.0, pain: 0.2]
│   ├── Left Eye                  [cognition: 0.15, pain: 0.9, hit_weight: 0.08]
│   ├── Right Eye                 [cognition: 0.15, pain: 0.9, hit_weight: 0.08]
│   ├── Left Ear                  [hit_weight: 0.05, pain: 0.4]
│   ├── Right Ear                 [hit_weight: 0.05, pain: 0.4]
│   ├── Nose                      [hit_weight: 0.06, pain: 0.6, bleed: 1.2]
│   ├── Jaw                       [hit_weight: 0.10, pain: 0.5]
│   └── Neck                      [bleed: 2.5, vital: false but VERY dangerous]
│
├── Torso                         [region: Torso, hit_weight: 1.0]
│   ├── Ribcage                   [armor: 0.3]
│   │   ├── Heart                 [vital: true, bleed: 5.0, hit_weight: 0.08]
│   │   ├── Left Lung             [breathing: 0.5, vital: false*, hit_weight: 0.12]
│   │   └── Right Lung            [breathing: 0.5, vital: false*, hit_weight: 0.12]
│   ├── Abdomen                   [hit_weight: 0.30]
│   │   ├── Stomach               [pain: 0.6, hit_weight: 0.10]
│   │   ├── Liver                 [bleed: 1.8, hit_weight: 0.10]
│   │   ├── Left Kidney           [hit_weight: 0.05]
│   │   ├── Right Kidney          [hit_weight: 0.05]
│   │   └── Intestines            [infection_factor: 3.0, hit_weight: 0.15]
│   ├── Spine                     [hit_weight: 0.05, locomotion: 1.0, can_amputate: false]
│   └── Pelvis                    [hit_weight: 0.10, locomotion: 0.3]
│
├── Left Arm                      [region: LeftArm]
│   ├── Left Shoulder             [hit_weight: 0.25, manipulation: 0.1]
│   ├── Left Upper Arm            [hit_weight: 0.25, bleed: 1.0]
│   ├── Left Elbow                [hit_weight: 0.10, manipulation: 0.15, pain: 0.7]
│   ├── Left Forearm              [hit_weight: 0.20, bleed: 0.8]
│   └── Left Hand                 [hit_weight: 0.20, manipulation: 0.4]
│       ├── Left Thumb            [manipulation: 0.15, hit_weight: 0.04]
│       ├── Left Index Finger     [manipulation: 0.10, hit_weight: 0.04]
│       ├── Left Middle Finger    [manipulation: 0.05, hit_weight: 0.04]
│       ├── Left Ring Finger      [manipulation: 0.03, hit_weight: 0.04]
│       └── Left Pinky            [manipulation: 0.02, hit_weight: 0.04]
│
├── Right Arm                     [region: RightArm — mirror of Left Arm]
│   └── (same structure, "Right" prefix)
│
├── Left Leg                      [region: LeftLeg]
│   ├── Left Hip                  [hit_weight: 0.15, locomotion: 0.15]
│   ├── Left Thigh                [hit_weight: 0.30, bleed: 1.5, locomotion: 0.1]
│   │   └── Left Femoral Artery   [bleed: 4.0, hit_weight: 0.05 — hidden, hit via thigh]
│   ├── Left Knee                 [hit_weight: 0.10, locomotion: 0.20, pain: 0.8]
│   ├── Left Shin                 [hit_weight: 0.20, locomotion: 0.05]
│   └── Left Foot                 [hit_weight: 0.20, locomotion: 0.15]
│       ├── Left Toes (grouped)   [locomotion: 0.05, hit_weight: 0.10]
│       └── Left Ankle            [locomotion: 0.15, hit_weight: 0.10, pain: 0.6]
│
└── Right Leg                     [region: RightLeg — mirror of Left Leg]
    └── (same structure, "Right" prefix)
```

*Both lungs destroyed = death (suffocation). One lung = severe breathing impairment but survivable.

### Hit Resolution: Two-Phase

**Phase 1 — Geometric (from collision point):**

```rust
// Transform hit to body-local coordinates
let dx = hit_x - entity_x;
let dy = hit_y - entity_y;
let local_x = dx * angle.cos() + dy * angle.sin();
let local_y = -dx * angle.sin() + dy * angle.cos();

// Determine coarse region from geometry
let region = if local_y > 0.15 && local_x.abs() < 0.08 {
    HitRegion::Head
} else if local_x < -0.12 {
    HitRegion::LeftArm
} else if local_x > 0.12 {
    HitRegion::RightArm
} else if local_y < -0.1 && local_x < 0.0 {
    HitRegion::LeftLeg
} else if local_y < -0.1 {
    HitRegion::RightLeg
} else {
    HitRegion::Torso
};
```

**Phase 2 — Weighted random within region:**

```rust
// Collect all parts in this region
let candidates: Vec<&BodyPartDef> = body_template.parts.iter()
    .filter(|p| p.region == region)
    .collect();

// Weighted random selection
let total_weight: f32 = candidates.iter().map(|p| p.hit_weight).sum();
let mut roll = rng.next_f32() * total_weight;
for part in &candidates {
    roll -= part.hit_weight;
    if roll <= 0.0 {
        return part;  // this specific part was hit
    }
}
```

**Phase 3 — Penetration depth (sub-part resolution):**

High-energy projectiles can penetrate outer parts and hit inner ones:
- Bullet hits `Ribcage` → if energy > ribcage armor threshold → continues to `Heart` or `Lung`
- Bullet hits `Skull` → if energy > skull armor → continues to `Brain`
- Bullet hits `Left Thigh` → if energy high → can hit `Femoral Artery` (hidden sub-part)

```rust
fn resolve_penetration(part: &BodyPartDef, energy: f32, rng: &mut Xorshift32) -> &BodyPartDef {
    if part.children.is_empty() || energy < part.armor * 100.0 {
        return part; // stopped here
    }
    // Penetrated — roll for child part
    let remaining_energy = energy - part.armor * 100.0;
    let child_candidates: Vec<_> = part.children.iter()
        .filter(|c| c.hit_weight > 0.0)
        .collect();
    if child_candidates.is_empty() { return part; }
    // ... weighted random among children, recurse ...
}
```

This means a rifle shot to the chest might: hit Torso → Ribcage (armor absorbs some) → penetrates → Heart (lethal) or Lung (critical). A pistol at range: hit Torso → Ribcage → stopped (fracture to ribs, no organ damage).

### bodies.toml

```toml
# Humanoid body template
[template.humanoid]
name = "Humanoid"

[[template.humanoid.part]]
id = "head"
name = "Head"
region = "head"
hit_weight = 1.0
max_hp = 20.0
pain_factor = 1.2
bleed_factor = 1.5
armor = 0.0
vital = false
can_amputate = false

[[template.humanoid.part]]
id = "skull"
name = "Skull"
parent = "head"
region = "head"
hit_weight = 0.0      # not directly hittable — only via penetration
max_hp = 30.0
armor = 0.4            # bone
vital = false

[[template.humanoid.part]]
id = "brain"
name = "Brain"
parent = "skull"
region = "head"
hit_weight = 0.0      # only via skull penetration
max_hp = 10.0
pain_factor = 0.2
vital = true           # destruction = instant death
cognition = 1.0

[[template.humanoid.part]]
id = "left_eye"
name = "Left Eye"
parent = "head"
region = "head"
hit_weight = 0.08
max_hp = 3.0
pain_factor = 2.0
cognition = 0.15       # losing an eye reduces accuracy/perception
can_amputate = false   # can't remove, but can be destroyed

# ... (60+ parts defined similarly) ...

[[template.humanoid.part]]
id = "left_index_finger"
name = "Left Index Finger"
parent = "left_hand"
region = "left_arm"
hit_weight = 0.04
max_hp = 3.0
manipulation = 0.10
pain_factor = 0.5
can_amputate = true

# Creature templates
[template.duskweaver]
name = "Duskweaver"

[[template.duskweaver.part]]
id = "body"
name = "Body"
region = "torso"
hit_weight = 0.6
max_hp = 30.0
vital = true
# ... etc
```

### Expandability

Adding a new body part requires ONLY a new entry in `bodies.toml`:

```toml
[[template.humanoid.part]]
id = "appendix"
name = "Appendix"
parent = "abdomen"
region = "torso"
hit_weight = 0.02     # very small, rarely hit
max_hp = 5.0
infection_factor = 5.0 # rupture causes severe infection
pain_factor = 1.5
```

No Rust code changes. The body tree is loaded once at startup and cached. Wound creation, healing, infection, treatment — all operate on `BodyPartDef` properties, not hardcoded part names.

### Derived Function Scores

Function is computed by walking the body tree and summing contributions:

```rust
fn locomotion_score(body: &BodyState, template: &BodyTemplate) -> f32 {
    template.parts.iter()
        .filter(|p| p.locomotion > 0.0)
        .map(|p| p.locomotion * part_effectiveness(body, p.id))
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

fn manipulation_score(body: &BodyState, template: &BodyTemplate) -> f32 {
    // same pattern with manipulation
}

fn cognition_score(body: &BodyState, template: &BodyTemplate) -> f32 {
    // same — eyes, brain contribute
}

fn breathing_score(body: &BodyState, template: &BodyTemplate) -> f32 {
    // lungs, trachea
    // both lungs at 0 = death (suffocation)
}
```

This means losing three fingers reduces manipulation by 0.18 — noticeable but not crippling. Losing the whole hand reduces it by 0.4. Losing the arm at the shoulder reduces it by the full arm's contribution. The math handles any combination of injuries naturally because it's additive from the tree.

### Creatures

Creatures use the same system with different templates:

```toml
[template.duskweaver]
# 8 parts: body, head, 6 legs
# Each leg has locomotion = 0.16 (losing 3 legs = 50% speed)

[template.thermogast]
# Armored body (armor: 0.6), vulnerable joints, thick skull
# Must hit joints or eyes to deal real damage — body shots bounce off
```

A thermogast hunt becomes tactical: "aim for the joints, body shots do nothing." This emerges from the data, not hardcoded AI.

## Wound Types

Each bullet impact creates a wound whose type depends on projectile energy, body part, and armor:

### Type Determination

```
kinetic_energy = 0.5 × mass × speed²
effective_energy = kinetic_energy × (1.0 - armor_absorption)

if effective_energy < 5.0:   → Scratch
if effective_energy < 20.0:  → Laceration
if effective_energy < 60.0:  → Puncture
if effective_energy < 150.0: → Fracture
else:                        → Shatter
```

Explosion damage → Burns (always, scaled by proximity).

### Wound Properties

| Type | Bleeding | Pain | Function Loss | Infection Risk | Heal Time |
|------|----------|------|---------------|----------------|-----------|
| Scratch | 0.001/s (stops in 30s) | 0.05 | 0% | 5% | Hours |
| Laceration | 0.005/s (needs bandage) | 0.15 | 5-10% | 15% | Days |
| Puncture | 0.015/s (needs pressure) | 0.30 | 20-40% | 30% | 1-2 weeks |
| Fracture | 0.008/s (internal) | 0.50 | 50-80% | 10% | 3-6 weeks |
| Shatter | 0.025/s (severe) | 0.80 | 90-100% | 40% | Months/never |
| Burn | 0.002/s | 0.40 | 20-50% | 50% | 1-3 weeks |

## Data Model

### Core Structs

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyPart {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WoundType {
    Scratch,
    Laceration,
    Puncture,
    Fracture,
    Shatter,
    Burn,
}

#[derive(Clone, Debug)]
pub struct Wound {
    pub body_part: BodyPart,
    pub wound_type: WoundType,
    pub severity: f32,          // 0.0-1.0 (scales all effects)
    pub bleeding_rate: f32,     // blood volume lost per second
    pub pain: f32,              // pain contribution (0.0-1.0)
    pub infection: f32,         // 0.0 = clean, 1.0 = sepsis
    pub infection_rate: f32,    // progression per second (0 if treated)
    pub healing: f32,           // 0.0 = fresh, 1.0 = fully healed
    pub healing_rate: f32,      // progress per second (affected by treatment, nutrition)
    pub treated: bool,          // bandaged/splinted
    pub foreign_body: bool,     // bullet fragment inside (needs extraction)
    pub permanent_damage: f32,  // residual function loss after healing (0-1)
    pub age: f32,               // seconds since wound
}

pub struct BodyState {
    pub blood: f32,             // 0.0-1.0 (0.3 = critical, 0.0 = dead)
    pub wounds: Vec<Wound>,     // all active wounds
    pub pain_total: f32,        // derived: sum of wound pains (clamped 0-1)
    pub consciousness: f32,     // derived: from blood + pain + head wounds
}
```

### Pleb Integration

Add `pub body: BodyState` to the `Pleb` struct. The existing `needs.health` becomes derived:

```rust
// health is now a derived value, not stored directly
pub fn effective_health(body: &BodyState) -> f32 {
    let blood_factor = body.blood;
    let wound_factor = 1.0 - body.wounds.iter()
        .map(|w| w.severity * (1.0 - w.healing))
        .sum::<f32>().min(1.0);
    let infection_factor = 1.0 - body.wounds.iter()
        .map(|w| w.infection * 0.5)
        .sum::<f32>().min(1.0);
    (blood_factor * wound_factor * infection_factor).clamp(0.0, 1.0)
}
```

## Blood System

Blood is a depletable resource. Every bleeding wound drains it.

```
blood_loss_per_second = Σ(wound.bleeding_rate × (1.0 - wound.healing))
body.blood -= blood_loss_per_second × dt
body.blood = body.blood.clamp(0.0, 1.0)

// Natural regeneration (slow, only when well-fed and resting)
if well_fed && resting {
    body.blood += 0.001 × dt;  // ~16 minutes from 0.5 to 1.0
}
```

**Blood loss thresholds:**

| Blood Level | State | Effects |
|-------------|-------|---------|
| 1.0 - 0.7 | Normal | None, slight pallor in sprite |
| 0.7 - 0.5 | Woozy | -20% accuracy, -15% work speed, occasional stumble |
| 0.5 - 0.3 | Severe | -50% everything, can barely stand, blurred vision |
| 0.3 - 0.1 | Critical | Collapse, unconscious, can't act |
| < 0.1 | Death | Dead without immediate transfusion |

**Blood on the ground:** Each tick, a bleeding entity drops blood at its position. Render as small dark red circles in the shader (similar to ground item rendering). Blood stains persist for hours, gradually darken.

## Pain and Shock

Pain is the sum of all wound pain values, modified by wound age (fresh wounds hurt more):

```rust
fn compute_pain(wounds: &[Wound]) -> f32 {
    wounds.iter()
        .filter(|w| w.healing < 1.0)
        .map(|w| {
            let freshness = (1.0 - (w.age / 300.0).min(1.0)) * 0.5 + 0.5; // fades from 1.0 to 0.5 over 5 min
            w.pain * w.severity * (1.0 - w.healing) * freshness
        })
        .sum::<f32>()
        .min(1.0)
}
```

**Pain thresholds:**

| Pain | State | Effects |
|------|-------|---------|
| 0.0 - 0.2 | Discomfort | Minor mood penalty |
| 0.2 - 0.4 | Pain | -10% work speed, grunt sounds |
| 0.4 - 0.6 | Agony | -30% work speed, can't do precise work |
| 0.6 - 0.8 | Severe | -50% everything, moaning (sound source), movement impaired |
| 0.8 - 1.0 | Shock | Collapse, unconsciousness, body shuts down |

**Shock** is a sudden consciousness drop from extreme pain. A single catastrophic wound (shatter, headshot) can cause immediate shock regardless of blood level. Shock resolves slowly (minutes) or with medical intervention.

## Infection

Untreated wounds accumulate infection over time:

```rust
fn tick_infection(wound: &mut Wound, dt: f32, environment: &EnvSample) {
    if wound.healing >= 1.0 || wound.infection >= 1.0 { return; }

    let base_rate = match wound.wound_type {
        Puncture => 0.0003,  // ~30% chance per hour
        Shatter => 0.0004,
        Burn => 0.0005,
        Laceration => 0.00015,
        _ => 0.00005,
    };

    let treatment_mult = if wound.treated { 0.1 } else { 1.0 };
    let foreign_mult = if wound.foreign_body { 3.0 } else { 1.0 };
    let env_mult = if environment.is_indoors { 0.7 } else { 1.0 }; // outdoors = dirtier

    wound.infection += base_rate * treatment_mult * foreign_mult * env_mult * dt;
    wound.infection = wound.infection.min(1.0);
}
```

**Infection progression:**

| Level | State | Effects |
|-------|-------|---------|
| 0.0 - 0.1 | Clean | None |
| 0.1 - 0.3 | Mild | Redness, warmth, slight fever, +pain |
| 0.3 - 0.6 | Moderate | Fever (affects work), pus, increasing pain |
| 0.6 - 0.8 | Severe | High fever, delirium (can't work), spreading |
| 0.8 - 1.0 | Sepsis | Organ failure, very likely death |

**Treatment vs infection:**
- Bandage: reduces infection rate ×0.1
- Herbal poultice: reduces existing infection by 0.1, slows rate
- Alcohol/whiskey: crude disinfection (-0.15 infection, painful)
- Medicine: stops mild infection, slows severe
- Surgery: removes foreign body (main infection source)
- Amputation: removes the infected limb entirely (last resort)

## Healing

Wounds heal naturally over time, faster with treatment:

```rust
fn tick_healing(wound: &mut Wound, dt: f32, nutrition: f32, resting: bool) {
    if wound.infection > 0.3 { return; } // infected wounds don't heal
    if wound.healing >= 1.0 { return; }

    let base_rate = match wound.wound_type {
        Scratch => 0.003,    // heals in ~5 minutes
        Laceration => 0.0005, // heals in ~30 minutes (game time)
        Puncture => 0.0001,   // heals in ~3 hours
        Fracture => 0.00003,  // heals in ~9 hours
        Shatter => 0.00001,   // heals in ~28 hours
        Burn => 0.00008,      // heals in ~3.5 hours
    };

    let treatment_mult = if wound.treated { 2.0 } else { 1.0 };
    let nutrition_mult = nutrition.clamp(0.3, 1.0); // starving = slow healing
    let rest_mult = if resting { 2.0 } else { 1.0 };
    let foreign_mult = if wound.foreign_body { 0.1 } else { 1.0 }; // barely heals with fragment inside

    wound.healing += base_rate * treatment_mult * nutrition_mult * rest_mult * foreign_mult * dt;

    // Bleeding decreases as wound heals
    wound.bleeding_rate *= 1.0 - wound.healing;

    // Pain decreases (but permanent damage remains)
    wound.pain = wound.pain * (1.0 - wound.healing * 0.8);
}
```

### Permanent Effects

When a wound heals (healing ≥ 1.0), it may leave permanent damage:

```rust
fn finalize_wound(wound: &mut Wound) {
    wound.permanent_damage = match wound.wound_type {
        Scratch => 0.0,
        Laceration => if wound.severity > 0.7 { 0.02 } else { 0.0 },
        Puncture => wound.severity * 0.05,
        Fracture => wound.severity * 0.15, // fractures leave lasting weakness
        Shatter => wound.severity * 0.40,  // shatters are devastating
        Burn => wound.severity * 0.10,
    };
}
```

**Permanent effects manifest as:**
- Scars (visible on sprite — per-body-part tint)
- Chronic pain (random flare-ups in cold weather)
- Reduced function (permanent_damage applied to body part effectiveness)
- Limp (leg with >0.1 permanent damage → visible movement speed reduction)
- Tremor (arm with >0.15 permanent damage → accuracy penalty)
- PTSD (head wound or multiple wounds → combat sounds trigger stress)

## Body Part Function Map

Each wounded part affects specific capabilities:

```rust
fn part_effectiveness(body: &BodyState, part: BodyPart) -> f32 {
    let wound_penalty: f32 = body.wounds.iter()
        .filter(|w| w.body_part == part && w.healing < 1.0)
        .map(|w| w.severity * (1.0 - w.healing) * function_loss(w.wound_type))
        .sum();
    let permanent_penalty: f32 = body.wounds.iter()
        .filter(|w| w.body_part == part)
        .map(|w| w.permanent_damage)
        .sum();
    (1.0 - wound_penalty - permanent_penalty).clamp(0.0, 1.0)
}
```

| Part | Affects | When Destroyed (effectiveness = 0) |
|------|---------|-------------------------------------|
| Head | Consciousness, accuracy, intellectual work | Instant death or permanent coma |
| Torso | Everything (×0.5 multiplier), breathing, internal organs | Death |
| Right Arm | Shooting accuracy, melee, crafting speed, hauling | Can't shoot, halved work speed |
| Left Arm | Secondary grip, hauling, crafting assist | Reduced hauling, slower crafting |
| Right Leg | Movement speed (×0.5), dodge | Crawling only, no running |
| Left Leg | Movement speed (×0.5), dodge | Crawling only, no running |

**Derived stats:**
```
movement_speed = base × min(left_leg_eff, right_leg_eff)^0.5 × blood_factor
work_speed = base × avg(right_arm_eff, left_arm_eff) × torso_eff × consciousness
accuracy = base × right_arm_eff × head_eff × blood_factor
```

Both legs at 50% = movement at 71% (sqrt). One leg at 0% = crawling (10% speed). This prevents a single leg wound from being as bad as two.

## Treatment System

### Treatment Actions

| Action | Who Can Do It | Time | Materials | Effect |
|--------|--------------|------|-----------|--------|
| **Apply pressure** | Anyone | Ongoing (hold) | None | Halves bleeding while held (can't do other work) |
| **Bandage** | Anyone | 10s | 1 fiber | Stops 80% bleeding, treated=true, reduces infection rate |
| **Splint** | Anyone (leg/arm) | 20s | 1 wood + 1 fiber | Stabilizes fracture, prevents severity increase |
| **Herbal poultice** | Anyone | 15s | 1 herb (future item) | -0.1 infection, +50% healing rate |
| **Clean wound** | Doc skill | 20s | Water | -0.15 infection risk, treated=true |
| **Extract fragment** | Doc skill | 60s | Medical tools (future) | foreign_body=false, allows proper healing |
| **Surgery** | Doc skill (high) | 120s | Medical tools + medicine | Repairs fractures properly, -0.2 severity |
| **Amputation** | Doc skill | 90s | Tools + fire source | Removes limb — total function loss but stops infection/bleeding |
| **Cauterize** | Anyone + fire | 5s | Campfire/torch nearby | Emergency: stops all bleeding, +0.3 burn damage, painful |

### Treatment Priority AI

Plebs with medical tasks prioritize:
1. Apply pressure to heavily bleeding allies (immediate)
2. Bandage all open wounds (quick, stops bleeding)
3. Extract foreign bodies (prevents chronic infection)
4. Treat infections (clean + poultice)
5. Surgery for fractures (improves long-term outcome)

## Creature Wounds

Creatures use the same wound system but with simplified anatomy:

```toml
# In creatures.toml
[[creature]]
id = 0
name = "Duskweaver"
body_parts = ["body", "legs"]    # hit: 60% body, 40% legs
body_armor = 0.0                 # no armor
leg_count = 6                    # losing legs reduces speed proportionally
lethal_body_damage = 30.0        # health threshold for body part

[[creature]]
id = 2  # future
name = "Thermogast"
body_parts = ["body", "joints", "head"]
body_armor = 0.6                 # 60% damage reduction on body hits
lethal_body_damage = 200.0
```

Duskweaver leg hits: each leg wound reduces speed by ~16%. Three legs hit = 50% speed. Body hit: standard wound → bleeding → death.

## Visual Representation

### Sprite Tinting

Per-body-part blood tint in the shader:

```wgsl
// In pleb rendering section
let wound_tint = pleb_wound_data[pi];  // packed: R=torso_blood, G=arm_blood, B=leg_blood, A=head_blood

// When rendering body part:
if in_torso_area {
    let blood = wound_tint.r;
    color = mix(color, vec3(0.4, 0.05, 0.02), blood * 0.6);
}
```

This requires a small per-pleb GPU buffer extension (4 floats for blood tint per body region).

### Blood Pools

Bleeding entities leave blood drops at their position. Rendered as small dark circles that persist and gradually darken over hours. Could be ground items or a separate blood_stains buffer.

### Wound Status UI

When a pleb is selected, show a body diagram in the info panel:
```
   [HEAD: OK]
[L.ARM: ▮▮▯] [TORSO: ▮▮▮▮] [R.ARM: ▮▮▮]
   [L.LEG: ▮▮] [R.LEG: ▮▮▮▮]

Wounds:
• Right arm: Puncture (healing 30%) — bandaged
• Left leg: Fracture (fresh) — BLEEDING — needs splint
Blood: ████████░░ 78%
Pain: ███░░░░░░░ 30%
```

## Down States (Revised from COMBAT.md)

The existing COMBAT.md down system, now driven by wounds:

| State | Trigger | Behavior |
|-------|---------|----------|
| **Healthy** | blood > 0.7, pain < 0.3 | Normal function |
| **Wounded** | blood 0.5-0.7 or pain 0.3-0.6 | Reduced capability, visible blood, grunts |
| **Critical** | blood 0.3-0.5 or pain 0.6-0.8 | Crawling, can't fight, calls for help (sound) |
| **Downed** | blood < 0.3 or pain > 0.8 | Unconscious, bleeding out. Minutes to stabilize. |
| **Dead** | blood = 0 or head destroyed | Permanent. Corpse. Colony-wide mood penalty. |

The key insight: **death is not instant.** A bullet to the torso creates a puncture wound → bleeding → blood loss over minutes → eventually death IF untreated. This creates the rescue window that COMBAT.md describes — fighting through to reach a downed friend.

Only a headshot (shatter to head) or massive torso damage causes near-instant death.

## Implementation Phases

### Phase 1: Data Model + Hit Location
- `BodyPart`, `WoundType`, `Wound`, `BodyState` structs (new `wound.rs` or extend `pleb.rs`)
- Hit location computation from collision point + entity angle
- Wound creation from projectile energy + body part
- Replace flat `health -= damage` with wound creation
- **Files**: wound.rs (new), physics.rs, simulation.rs, pleb.rs

### Phase 2: Bleeding + Blood
- `blood` field on BodyState
- Per-wound bleeding ticked each frame
- Blood loss effects on movement/accuracy/consciousness
- Bandage action (anyone, fiber cost)
- Death from blood loss
- **Files**: wound.rs, simulation.rs, needs.rs

### Phase 3: Pain + Shock
- Pain computation from wounds
- Consciousness from blood + pain + head wounds
- Pain thresholds affecting work speed, accuracy
- Shock from catastrophic single wounds
- Moaning sound source from high pain
- **Files**: wound.rs, simulation.rs, audio integration

### Phase 4: Infection
- Infection ticking on untreated wounds
- Foreign body (bullet fragments) accelerating infection
- Infection stages with escalating effects
- Basic treatment: clean wound, herbal poultice
- **Files**: wound.rs, simulation.rs

### Phase 5: Healing + Treatment
- Natural healing rate modifiers (nutrition, rest, treatment)
- Treatment actions as pleb activities
- Doc skill for advanced treatment
- Permanent damage from healed severe wounds
- **Files**: wound.rs, simulation.rs, pleb.rs

### Phase 6: Visual + UI
- Per-body-part blood tint in shader
- Blood pools/stains on ground
- Wound status panel in selection info
- Body diagram UI
- **Files**: raytrace.wgsl, ui.rs, main.rs

### Phase 7: Creature Wounds
- Simplified body parts for creatures
- Leg hits reduce creature speed
- Wound-based death instead of flat HP
- **Files**: creatures.rs, wound.rs, simulation.rs

## Connections to Other Systems

| System | Connection |
|--------|-----------|
| **DN-011 Combat Rework** | Collision system provides hit position for body part determination |
| **COMBAT.md** | Wound system implements the damage model + down states described there |
| **DEEPER_SYSTEMS.md** | Medical treatment progression (rest → herbs → frontier medicine → surgery) |
| **CHARACTER_VISUALS.md** | Scars, blood stains, bandage sprites, limp animation |
| **LORE_AND_RESEARCH.md** | Medical knowledge as lore items (wound treatment techniques, surgical procedures) |
| **Needs system** | Health becomes derived; pain affects mood; blood loss affects fatigue |
| **Sound system** | Pain moaning, "medic!" callout, bone crack on fracture |
| **Creature system** | Creatures use same wound model with simplified anatomy |

## Design Philosophy

From PHILOSOPHY.md: "Not punishing — consequential."

- A bullet wound isn't instant death. It's a problem that demands a response.
- An untreated wound escalates. Ignoring injuries kills people slowly.
- Treatment takes time and resources. The doc can't be everywhere.
- Permanent damage makes combat COSTLY even when you win.
- Every scar tells a story. The map of a colonist's body is a record of what they survived.
- The choice to amputate a gangrenous limb to save a life — that's the kind of decision this system creates.
