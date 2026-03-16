# Fluid Mechanics - Inspiration & References

## Demos

- **VERY HIGH**: https://paveldogreat.github.io/WebGL-Fluid-Simulation/
  - Full Navier-Stokes on WebGL. 16k stars on GitHub. MIT licensed.
  - Pipeline: curl → vorticity confinement → divergence → Jacobi pressure (20 iters) → gradient subtract → advect velocity → advect dye
  - Key features: separate sim resolution (128) vs dye resolution (1024), bloom, sunrays, vorticity confinement
  - Tunable: pressure iterations, curl strength, dissipation rates, splat radius/force
  - **This is our primary implementation reference.** The entire solver is in `script.js` (~1100 lines).
  - Source: https://github.com/PavelDoGreat/WebGL-Fluid-Simulation

- **VERY HIGH**: https://29a.ch/sandbox/2012/fluidwebgl/
  - WebGL Navier-Stokes by Jonas Wagner. Based on Jos Stam's "Stable Fluids" paper.
  - Simpler than PavelDoGreat (no vorticity confinement, no bloom), but clean architecture.
  - Good reference for the minimal viable pipeline: advect → forces → divergence → pressure → gradient subtract.
  - Blog post with data flow diagram: https://29a.ch/2012/12/16/webgl-fluid-simulation
  - Source: https://github.com/jwagner/fluidwebgl

- **HIGH**: https://onsetsu.github.io/floom/example.html
  - MPM (Material Point Method) fluid simulation. Particle-based / Lagrangian approach.
  - Handles multiple materials with different properties, elastic materials via springs, surface tension.
  - Relevant for future Tier 2 work: liquid water, lava, mud, deformable terrain.
  - Not for the prototype — MPM is a different solver than Eulerian NS. But architecturally compatible (MPM uses the same grid for particle-to-grid transfer).
  - Source: https://github.com/onsetsu/floom

- **HIGH**: https://prideout.net/blog/old/blog/index.html@p=58.html
  - Philip Rideout's "Simple Fluid Simulation" tutorial. Classic GPU Gems Eulerian grid approach.
  - Great pedagogical value: explains advection, buoyancy, divergence, Jacobi, gradient subtract step by step.
  - Includes C/GLSL source code (public domain).
  - Covers obstacle handling with a dedicated obstacle texture (R = solid, GB = obstacle velocity).
  - Graphviz diagrams of the full pipeline with and without obstacles.
  - Good for understanding the "why" behind each step.

## Academic / Foundational References

- **Jos Stam, "Stable Fluids" (1999)**: The foundational paper. Semi-Lagrangian advection that is unconditionally stable. Everything above is based on this.
  - PDF: http://www.dgp.toronto.edu/people/stam/reality/Research/pdf/ns.pdf

- **GPU Gems Chapter 38, "Fast Fluid Dynamics Simulation on the GPU" (Mark Harris, 2004)**: The GPU implementation guide. Maps Stam's method to fragment shaders.
  - Online: https://developer.nvidia.com/gpugems/gpugems/part-vi-beyond-triangles/chapter-38-fast-fluid-dynamics-simulation-gpu

- **Game Physics (David H. Eberly)**: Comprehensive reference for physics simulation including fluids.
