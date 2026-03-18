//! Cross-platform time — uses std::time on native, Performance.now() on WASM.

#[derive(Clone, Copy)]
pub struct Instant(
    #[cfg(not(target_arch = "wasm32"))]
    std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    f64,
);

impl Instant {
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        { Instant(std::time::Instant::now()) }

        #[cfg(target_arch = "wasm32")]
        {
            let perf = web_sys::window()
                .expect("no window")
                .performance()
                .expect("no performance");
            Instant(perf.now())
        }
    }

    pub fn elapsed_secs_since(&self, earlier: &Instant) -> f32 {
        #[cfg(not(target_arch = "wasm32"))]
        { (self.0 - earlier.0).as_secs_f32() }

        #[cfg(target_arch = "wasm32")]
        { ((self.0 - earlier.0) / 1000.0) as f32 }
    }
}
