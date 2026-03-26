# Art Style Directions

Six distinct art directions for Rayworld. The art style lives in the intersection of palette, shading, post-processing, and UI — all achievable through the existing raycasting shader pipeline, per-block render styles, and post-processing.

## 1. "Dust & Iron" — Gritty Spaghetti Western

The Leone look. Squinting eyes, scorching sun, blood in the dirt.

**Palette:** Desaturated warmth — ochre, rust, burnt sienna, dried blood red, dusty cream. Black shadows. No pure whites except the noon sun.

**Rendering:** Heavy contrast. Shadows are nearly black. Surfaces are weathered — noise-based grime on every material. Wood has visible grain. Metal has patina. Nothing is clean.

**Post-processing:** Film grain (subtle, always on). Slight vignette darkening edges. Heat shimmer near fire and during midday. Occasional dust motes floating in light shafts.

**Lighting:** Golden hour is the hero moment — long shadows stretching across the map. Noon is harsh and washed out. Night is deep indigo with orange lamplight pools.

**UI:** Typewriter font. Sepia-toned panels. Wanted poster borders. Ink stamps for buttons. Event cards look like old telegrams.

**Mood:** Tense. Beautiful but dangerous. Every sunset could be your last.

## 2. "Moebius Frontier" — Surreal Sci-Fi Western

Inspired by Jean Giraud (Moebius) who literally drew both westerns (*Blueberry*) and sci-fi (*Arzach*, *The Incal*). The perfect bridge between the two genres.

**Palette:** Unexpected color combinations — purple skies over orange desert, teal shadows on pink sandstone, chartreuse alien vegetation. Colors that feel *wrong* but beautiful. This is an alien planet, not Earth.

**Rendering:** Clean, almost flat shading with dramatic rim lighting on edges. Blocks have crisp silhouettes. Minimal texture noise — color does the work. Strong outlines (1px black or dark complementary).

**Post-processing:** Minimal — the colors speak for themselves. Maybe a subtle chromatic aberration at screen edges to feel slightly alien.

**Lighting:** Colored light sources. Sunset isn't just orange — it's magenta bleeding into violet. Torchlight is amber against blue-purple night. Two moons casting different colored shadows.

**UI:** Clean sans-serif. Thin lines. Retro-futuristic — like a 1970s sci-fi paperback cover. Cards have clean geometric borders with subtle gradients.

**Mood:** Dreamlike. Vast. Lonely in a beautiful way. The frontier as alien wonder.

## 3. "Dime Novel" — Pulp Western Illustration

Old-school adventure pulp. Bold, loud, fun. Think penny dreadfuls and serialized frontier tales.

**Palette:** Limited — 5-6 base colors with 3 shades each. Strong primaries: red, yellow, deep blue. Black outlines. Like a hand-colored woodcut print.

**Rendering:** Bold 1-2px outlines on all blocks. Halftone dot patterns for shadows (achievable in shader with a threshold + dot grid). Flat color fills with crosshatch shading on depth faces. Slightly exaggerated proportions.

**Post-processing:** CMYK halftone overlay (subtle). Slight paper texture. Color bleed at edges like misregistered printing.

**Lighting:** Simplified — 3 tiers (lit, shadow, deep shadow) rather than smooth gradients. Dramatic spotlight effects during events. Comic-book "speed lines" during action.

**UI:** Hand-lettered fonts. Exclamation marks! Star bursts for notifications. Cards look like actual pulp novel covers with dramatic titles. Bold colored borders.

**Mood:** Exciting. Slightly campy. Every event feels like a cliffhanger chapter ending.

## 4. "Copper & Sage" — Naturalist Western

The Terrence Malick western. *Days of Heaven*, *The Assassination of Jesse James*. Quietly gorgeous.

**Palette:** Earth tones only — copper, sage green, sandstone tan, slate blue-grey, warm cream, bark brown. Accent colors are wildflowers: lupine purple, poppy orange, but rare and precious.

**Rendering:** Painterly. Terrain uses layered noise for a watercolor wash effect. Blocks have soft edges (anti-aliased or slightly blurred boundaries). Wood grain is lovingly detailed. Stone has visible strata layers.

**Post-processing:** Soft bloom on light sources. Gentle depth-of-field blur at screen edges. Color grading shifts with time of day — cool blue mornings, warm amber afternoons, lavender dusk.

**Lighting:** Natural and gentle. No harsh shadows — soft penumbra. Magic hour (dawn/dusk) lasts longer and is gorgeous. Overcast days have a flat, silvery quality.

**UI:** Leather journal aesthetic. Handwritten-style font. Stitched borders. Cards look like botanical illustrations or naturalist field sketches. Muted, warm tones.

**Mood:** Contemplative. Beautiful. Makes you stop and watch the sunset. The harshness of survival softened by the beauty of the land.

## 5. "Neon Frontier" — Cyberpunk Meets Cowboy

*Cowboy Bebop* meets *Blade Runner* meets *Firefly*. The future is dusty AND neon.

**Palette:** Dark base (charcoal, deep navy, black) with electric accents — cyan, hot magenta, acid green, amber. The natural world is muted; technology glows.

**Rendering:** Dark terrain with occasional neon signage. Power cables glow. Electrical systems emit visible light traces. Powered buildings have luminous trim. Unpowered areas are shadowy and analog.

**Post-processing:** Bloom on all light sources (heavy). Scanlines (very subtle). Chromatic aberration. Rain has visible streaks catching neon light. Reflective puddles.

**Lighting:** Extreme contrast. Deep shadows with neon spill. Torches are warm analog islands in a cold digital night. Stars are vivid — you can see the galaxy.

**UI:** Holographic feel — thin glowing lines, transparent panels, monospace font. Cards are data slates with glitch effects. Notifications pulse.

**Mood:** Stylish. High energy. The frontier as the edge of civilization where old meets new.

## 6. "Tin Type" — Living Daguerreotype

The entire game looks like an old photograph that's come alive. Haunting and unique.

**Palette:** Sepia is the base. Almost everything is brown/cream/amber monochrome. But — **selective color** breaks through: fire is orange, blood is red, the sky at sunset gets color, flowers bloom in muted pastels. Color is rare and therefore meaningful.

**Rendering:** High contrast with soft gradients. Slight blur/soft focus at edges (period lens simulation). Grain is always present — heavy, photographic grain, not film grain. Scratches and imperfections overlay.

**Post-processing:** Sepia color grading with selective color pass (only certain game elements get hue). Vignette (strong). Slight image instability — barely perceptible drift, like a projected photograph. Occasional "flash" when events trigger (old camera flash).

**Lighting:** Single dominant light source (sun) creates simple but dramatic shadows. Night is near-black with tiny warm pools from lamps. Lightning storms are spectacular — the whole screen flashes white.

**UI:** Handwritten cursive labels. Old paper texture. Wax seal stamps. Cards look like actual tin type photographs with ornate frames. Very tactile.

**Mood:** Haunting. Atmospheric. History being made. Every screenshot looks like a found artifact from a lost colony.

## Recommendation

For a space western colony sim with card mechanics, the strongest candidates are:

**Moebius Frontier** — The alien-planet setting justifies wild color choices, and the clean rendering style works well with the block-based raycaster. Most distinctive — no other colony sim looks like a Moebius comic.

**Dust & Iron / Tin Type hybrid** — Selective color (sepia world, colored fire/UI/cards) would make the card system pop visually. Cards burst with color in an otherwise muted world. Fire matters more when it's the brightest thing on screen.

The card system benefits from whichever style makes the cards themselves *feel special* when they appear. In a Moebius world, cards could be ornate and geometric. In a Tin Type world, cards are the only full-color elements — they literally feel like magic.
