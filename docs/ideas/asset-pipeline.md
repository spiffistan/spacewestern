# Asset Production Pipeline

Tools, services, and workflows for creating Rayworld's sound, art, and music assets. Organized by production phase: AI-first for rapid prototyping, then refinement paths toward production quality.

## Guiding Principle

AI generates the first draft. Humans refine the final product. Use AI to explore 50 variations in an afternoon, pick the 5 that work, hand-polish those into shipping assets. For a solo/small team, this is the only way to produce the volume of assets a colony sim demands without years of manual work.

---

## Sound Effects

The game needs hundreds of distinct sounds: creature calls, tool impacts, weather, fire, footsteps on different surfaces, UI feedback, ambient environments, radio static, the hollowcall, construction, smelting, cooking, combat, and more.

### AI Generation (Prototyping Phase)

**ElevenLabs Sound Effects** — elevenlabs.io/sound-effects
The strongest general-purpose AI SFX generator as of 2026. Text-to-sound with good realism. Free tier available. Particularly good at environmental and mechanical sounds.
- Use for: creature vocalizations ("chittering insectoid clicking, nocturnal, eerie"), tool impacts ("hammer striking iron anvil, indoor workshop"), ambient environments ("night wind through rocky terrain, distant echoing call")
- Strength: the prompt system handles descriptive scenarios well, not just sound names
- Output: stereo, 44.1kHz, royalty-free for commercial use

**Stable Audio** — stableaudio.com
Good at atmospheric and textured sounds. Open model weights available for local generation.
- Use for: ambient loops ("desolate alien wind, low frequency, vast empty landscape"), weather ("rain on metal roof, thunder distant"), fire ("campfire crackling, wood popping, contained")
- Strength: longer-form ambient generation, good at loops

**Meta AudioCraft** — github.com/facebookresearch/audiocraft
Open-source, runs locally. No cloud dependency, no per-generation cost. More technical setup but unlimited free generation.
- Use for: bulk generation of variations (50 footstep variants, 30 tool impacts)
- Strength: free, local, scriptable for batch generation

**OptimizerAI** — optimizerai.xyz
Situation-based prompting — describe the scene, not the sound. "A man walking through mud in heavy boots" rather than "mud footstep sound."
- Use for: complex layered scenes, Foley-style sounds
- Strength: contextual understanding, variation generation from a single prompt

### Free Sound Libraries (Production Quality)

For sounds that AI can't nail or where you want hand-recorded authenticity:

**Freesound.org** — The essential resource. 500,000+ sounds, filter by CC0 license for no-attribution-required use. Particularly strong for:
- Nature and environmental ambience
- Tool and construction sounds
- Weather (rain, wind, thunder recorded on location)
- Fire and water

**OpenGameArt.org** — CC0 game-specific SFX collections. Pre-organized for game use.

**Kenney.nl** — All assets CC0. Clean, well-organized game SFX packs. Interface sounds, impacts, ambient.

### Hybrid Approach (Recommended)

1. **Generate 10–20 AI variations** of each sound concept with ElevenLabs/Stable Audio
2. **Pick the best 2–3** that capture the right tone
3. **Layer and process** in a free DAW:
   - **Audacity** (free, open source) — basic cutting, layering, normalization
   - **Reaper** (free evaluation, $60 license) — professional DAW, excellent for game audio
4. **Supplement with Freesound CC0** for base layers (ambient beds, raw material sounds)
5. **Export as .ogg or .wav** at the game's sample rate

### Sound Design Priorities for Rayworld

The sound sim makes EVERY sound a gameplay element. Priority order:

| Priority | Category | Count | Approach |
|----------|----------|-------|----------|
| Critical | Creature calls (6 species) | ~30 | AI generate + heavy processing for alien feel |
| Critical | Ambient day/night loops | ~8 | Freesound CC0 nature beds + AI alien elements layered |
| High | Tool/work sounds (axe, pick, hammer, smelter) | ~40 | Freesound CC0 for base + AI for alien-material variants |
| High | Weather (rain, wind, thunder, snow) | ~20 | Freesound CC0 — real recordings are best |
| High | Fire (campfire, building fire, smelter, kiln) | ~15 | Freesound CC0 fire recordings |
| Medium | Footsteps (per surface: dirt, stone, wood, grass, mud, snow) | ~60 | AI batch generation per surface type |
| Medium | Combat (weapon fire, impact, reload) | ~30 | AI + Freesound mix |
| Medium | UI / interaction sounds | ~20 | AI generation, clean and processed |
| Low | Radio static, transmissions, voices | ~15 | ElevenLabs voice AI + static processing |
| Low | Music / instrument sounds | ~10 | See Music section below |
| Deep | Hollowcall and alien resonance | ~5 | Custom synthesis — see below |

### The Hollowcall (Special Case)

The hollowcall is the game's signature sound. It needs to be custom-designed, not generated:

- **Base:** A low-frequency drone (40–80Hz) that's felt as much as heard
- **Layer 1:** A whale-call-like harmonic sweep — organic but wrong, like a voice through water
- **Layer 2:** A metallic resonance — as if the sound is passing through vast metal pipes
- **Layer 3:** Subtle rhythmic pulsing — not a heartbeat, something more mechanical, like a failing oscillator

The hollowcall should be synthesizable at runtime from parameters (frequency, amplitude, harmonic content) so it can vary with seasons, distance, and ancient infrastructure activity. Tools for designing the base sound:
- **Vital** (free synth) — excellent for evolving drone textures
- **SuperCollider** (free, open source) — procedural audio synthesis, good for generating the runtime parameters
- **Audacity** spectral editing — for fine-tuning harmonic content

---

## Art and Sprites

The game needs: colonist sprites (layered, animated), creature sprites, terrain textures, item icons, building textures, UI elements, and potentially procedural materials for the raytracer.

### AI Sprite Generation (Prototyping Phase)

**PixelLab** — pixellab.ai
The strongest tool for game-specific pixel art as of 2026. Generates at specific pixel sizes, supports directional rotation (4-dir, 8-dir), and has animation generation.
- Use for: colonist sprites with directional variants, creature concepts, item icons
- Key feature: style locking — define a palette and art direction once, every sprite inherits it
- Integrates with Aseprite as a plugin
- Free tier + paid plans

**Sprixen** — sprixen.com
Automated pipeline: bulk sprite generation with palette enforcement, one-click animation (walk cycles, attacks, idles), 8-directional isometric turnarounds. $15/mo, 20 free credits.
- Use for: bulk asset generation (50 item icons in an afternoon, enemy variants)
- Key feature: API access for programmatic generation
- Exports as PNG sprite sheets for any engine

**Sprite-AI** — sprite-ai.art
Generates at exact game sizes (16×16 through 128×128). Built-in pixel editor for touch-ups. Free tier.
- Use for: quick concepts, item icons, placeholder art
- Strength: generates actual game-sized sprites, not "pixel art style" illustrations

**Midjourney / DALL-E / Stable Diffusion** (general-purpose)
Not pixel-art-specific but excellent for concept art, mood boards, and reference images:
- Use for: "What should the smelter look like?" "What's the aesthetic of Fort Morrow?" "Concept art for the thermogast"
- Then feed the concept into PixelLab or Sprixen as a reference image for pixel art generation
- Midjourney particularly good with prompts like "top-down pixel art colony, dust and iron aesthetic, warm amber lighting, space western"

### Manual Art Tools (Refinement Phase)

**Aseprite** ($20, one-time) — The industry standard pixel art editor. Animation timeline, onion skinning, palette management, tilemap support, Lua scripting for batch operations. Hero characters and signature sprites should be hand-refined here after AI generation.

**Piskel** (free, browser-based) — Lighter alternative to Aseprite. Good enough for quick edits and prototyping. No Lua scripting.

**Lospec Palette List** (free) — Curated pixel art palettes. Pick one that matches the space western vibe (earth tones, muted amber, steel blue, rust) and lock ALL AI generation to it.

### Art Workflow for Rayworld

The game uses a GPU raytracer with procedural materials — not traditional pre-rendered tile sprites. Most "art" is actually material parameters (color, roughness, emission) fed to the shader. But sprites ARE needed for:

1. **Colonist sprites** (DN-009) — layered compositing: body, clothing, equipment, hair. These are the most visible art assets. AI-generate body bases → hand-refine in Aseprite → export as sprite sheets.
2. **Creature sprites** — 6 species, each with idle/walk/attack/death animations. AI-generate concepts → hand-refine silhouettes → animate in Aseprite.
3. **Tree and vegetation sprites** — already exist (the 6292-line shader samples them). Replaceable/upgradable.
4. **Item icons** — for the equipment panel (DN-018) and crafting UI. 50–100 needed. AI-generate in bulk → hand-refine the bad ones.
5. **UI elements** — minimal (DN-020 philosophy), but buttons, icons, panel borders.
6. **Concept art** — for the art direction decision (artstyle.md). Generate 50 concepts in different styles → pick the direction → lock the palette.

The procedural material system means terrain, walls, floors, and most block types DON'T need sprites — they're defined by color values and material properties in the shader. This dramatically reduces the art asset count compared to a traditional tile-based game.

---

## Music and Ambient Score

### AI Music Generation (Prototyping Phase)

**Suno** — suno.com
Text-to-music with surprisingly good results for ambient, western, and atmospheric genres. Generates full songs or instrumental tracks from descriptions.
- Use for: "sparse acoustic guitar, desert wind, lonely frontier" → prototype soundtrack
- Strength: can generate in very specific moods and genres
- Licensing: check current terms — Suno's commercial licensing has evolved

**Soundverse** — soundverse.ai
Text-to-music with loop mode — generates seamless loops for game use. Good for ambient beds and level-specific atmosphere.
- Use for: ambient exploration music, seasonal mood loops, combat tension
- Key feature: Loop Mode ensures seamless in-game repetition

**Udio** — udio.com
Strong at genre-specific generation. Good for the space western sound specifically.
- Use for: saloon music (honky-tonk piano, frontier fiddle), dramatic tension cues, campfire atmosphere

**Beatoven.ai** — beatoven.ai
Designed specifically for content creators and game devs. Specify mood, tempo, genre, and get royalty-free tracks.
- Use for: quick mood-specific background tracks during development

### The Rayworld Soundtrack Approach

The game shouldn't have a traditional OST that loops. It should have **stems and layers** that the sound system mixes dynamically:

**Ambient bed** — always playing, nearly inaudible. Changes with season and time of day. Generated with Stable Audio or Soundverse as long loops.

**Tension layer** — fades in when threats are near. A low drone, subtle percussion. AI-generated, then processed for seamless crossfading.

**Activity layer** — light acoustic elements that respond to colony activity. More people working = richer texture. Quiet colony = sparse.

**Silence** — the most important "music." Long stretches with no music at all. Just the sound sim: wind, footsteps, the distant hollowcall, fire crackling. Silence IS the soundtrack. Music should feel like a rare event — when it plays, it means something.

**Diegetic music** — a colonist playing an instrument in the saloon (light-and-sound.md). This is the only "real" music — it comes from a source in the world and propagates through the sound sim. A single guitar or harmonica recording, looped with variation. Record or commission 3–4 short pieces for the saloon musician. These become the colony's most precious cultural artifacts.

### Voice

**ElevenLabs Voice** — elevenlabs.io
The leading AI voice platform. Can generate:
- Radio transmissions with specific voice characteristics ("tired woman, static-distorted, describing a storm")
- The Perdition broadcast (calm male voice, professional, 47 years of degradation layered on)
- Trader greetings, newcomer introductions
- Colonist barks ("Enemy spotted!", "I'm hit!", "Beautiful sunset")

For prototyping, AI voices are fine. For shipping, consider:
- Recording real voice actors for hero moments (the Perdition broadcast, key radio transmissions)
- Using AI for bulk ambient voices (colonist barks, background chatter, distant radio)
- Processing all voices through the game's audio pipeline (adding static, distance attenuation, wall muffling from the sound sim)

---

## Pipeline Summary

```
PROTOTYPING (weeks 1–4):
  Sound:  ElevenLabs + Freesound CC0 → Audacity processing → .ogg
  Art:    PixelLab/Sprixen + Midjourney concepts → Aseprite polish → .png
  Music:  Suno/Soundverse → loop/stem processing → .ogg

REFINEMENT (ongoing):
  Sound:  Replace worst AI sounds with Freesound or recordings
  Art:    Hand-refine hero sprites in Aseprite
  Music:  Commission or record saloon music, hollowcall design in Vital/SuperCollider
  Voice:  ElevenLabs for bulk, real actors for hero moments

SHIPPING:
  Sound:  Full pass — replace any remaining placeholder AI with polished assets
  Art:    Consistent palette, all sprites at final resolution
  Music:  Dynamic stem system implemented, silence-first design
  Voice:  All hero voices recorded, AI voices processed for consistency
```

### Rough Cost Estimate (Prototyping Phase)

| Tool | Cost | What You Get |
|------|------|-------------|
| ElevenLabs (sound + voice) | Free tier → $5–22/mo | SFX generation, voice generation |
| PixelLab or Sprixen | Free tier → $15/mo | Sprite generation with style locking |
| Aseprite | $20 one-time | Professional pixel editor |
| Reaper DAW | Free eval → $60 | Audio processing and mixing |
| Suno or Soundverse | Free tier → $10/mo | Music generation |
| Freesound/OpenGameArt/Kenney | Free (CC0) | Hundreds of production-quality sounds |
| Vital synth | Free | Hollowcall and drone design |
| **Total (prototyping)** | **~$30–60/mo + $80 one-time** | Full pipeline |

For a solo/small indie project, this is extraordinarily cheap compared to hiring sound designers ($50–150/hr), pixel artists ($30–80/hr), or composers ($500–5000/track).

---

## What Can't Be AI-Generated (Yet)

Some things need human craft from the start:

- **The hollowcall** — too specific, too important, too central to the game's identity. Custom synthesize it.
- **Procedural material parameters** — the raytracer's material system is code, not art assets. Color values, roughness, emission — these are tuned by hand in the shader.
- **The sound sim itself** — the wave equation is the "sound engine." AI doesn't generate physics simulations.
- **Art direction decisions** — AI can generate 50 options, but a human picks the one that FEELS right.
- **Palette selection** — choose it once, enforce it everywhere. This is a creative call, not a generation task.
