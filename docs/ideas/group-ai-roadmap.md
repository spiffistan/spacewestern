# Group AI & Communication Roadmap

Ideas building on the shout system (DN-013) and group commands. Ranked by impact-to-effort.

## 1. Information Propagation Delay

**Effort**: Low (30 lines) | **Impact**: Medium

Shouts travel at ~10 tiles/frame instead of instantly. Store in-flight shouts as `Vec<(Shout, f32)>` with expanding radius. Plebs react when the wavefront reaches them. Creates staggered cascade: first pleb spots enemy, 0.3s later second reacts, 0.5s later third. Pairs with GPU sound propagation (Phase 2 of DN-013 sound integration).

## 2. Squad Roles

**Effort**: Medium (50 lines) | **Impact**: Medium

When `form_group` is called, assign roles based on pleb skills:
- **Point** (highest melee): leads approach, always charges
- **Overwatch** (highest shooting): hangs back at cover, suppressive fire
- **Flanker** (highest speed/melee): takes perpendicular path (existing flanking offset)

Implementation: `squad_role: u8` on Pleb, set by `form_group` from skill comparison. Combat AI branches on role. Point men ignore cover-seeking. Overwatch always seeks cover. Flankers get 2x flanking offset.

## 3. Morale Contagion

**Effort**: Low (20 lines) | **Impact**: Medium

Stress spreads through shouts:
- Pleb mental-breaks → nearby allies +15 stress
- "Clear!" shout → nearby allies -5 stress
- Retreating enemy visible → friendly stress -3
- Dying friendly → nearby +20 stress

Creates cascading morale. One bad moment triggers chain breaks. One decisive kill rallies the team. All through existing shout + stress systems.

## 4. Enemy Approach Coordination

**Effort**: Medium (40 lines) | **Impact**: High

When an enemy with `group_id` spots a friendly, ALL same-group enemies get the target position via Alert shout. They coordinate:
- Melee enemies advance together (flocking keeps them grouped)
- Ranged enemies seek cover facing the target
- Group forms up at ~20 tiles, then advances as a wave

Missing piece: a "group advance" state where the first spotter's target becomes the group's shared destination. All group members path toward the same area, flocking keeps them coherent, then they disperse to cover on contact.

## 5. Radio/Horn Communication

**Effort**: High (60 lines + block def) | **Impact**: Medium

Buildable item: signal horn (low-tech) or radio (mid-tech). Extends shout range to map-wide for the carrier. The horn emits a loud unique sound (new audio pattern). Without a horn, shouts limited to 15-20 tiles. With it, whole colony hears.

Decisions: who carries the horn? What if the carrier dies? Enemy horns alert distant enemy groups to rally. Creates a "communications officer" role.

## 6. Surrender Mechanic

**Effort**: Medium (50 lines) | **Impact**: High

Enemies below 15% health with no allies within 10 tiles drop weapons and stop fighting. New `PlebActivity::Surrendering` — they pathfind toward nearest friendly with hands up (new sprite pose). Player decides: capture (becomes prisoner/laborer), release, or execute.

Requires: new activity state, new shout type ("I surrender!"), prisoner pleb state (non-enemy but restricted), and a decision UI. Opens a whole gameplay branch — labor, interrogation, trade.

## 7. Overwatch Mode

**Effort**: Medium (30 lines) | **Impact**: Medium

A stance toggle for drafted plebs: instead of actively targeting, the pleb watches a cone-shaped area and fires at the FIRST enemy that enters it. Like a turret — they don't seek targets, they wait.

Implementation: new `overwatch_angle: Option<f32>` on Pleb. When set, the combat code skips normal target acquisition and instead checks if any enemy is within the overwatch cone (±30 degrees of the set angle, 15 tile range). First enemy that enters gets instant fire (no aim windup, but wider spread). After firing, overwatch clears (one shot). Player sets overwatch by pressing O while drafted — uses current facing angle.

Creates ambush tactics: set up overwatch covering an approach, enemies walk into a kill zone.

## Implementation Priority

1. **Morale contagion** — 20 lines, immediately makes combat emotional
2. **Enemy approach coordination** — 40 lines, enemies feel like a real threat
3. **Information delay** — 30 lines, makes communication feel physical
4. **Squad roles** — 50 lines, depth for repeat players
5. **Overwatch mode** — 30 lines, tactical positioning matters
6. **Surrender** — 50 lines, opens prisoner gameplay
7. **Radio/horn** — 60 lines + assets, mid-game content
