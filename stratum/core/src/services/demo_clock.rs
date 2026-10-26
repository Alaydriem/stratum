// Picks the layer the demo shows from a running clock.
pub struct DemoClock {
    step_seconds: f64,
    start: Option<f64>,
}

impl DemoClock {
    pub fn new(step_seconds: f64) -> Self {
        Self { step_seconds, start: None }
    }

    // The first call starts the clock at `bottom`. The layer then rises one every step and wraps
    // to `bottom` after `top`.
    pub fn layer(&mut self, now: f64, bottom: i32, top: i32) -> i32 {
        let start = *self.start.get_or_insert(now);
        let count = i64::from(top.max(bottom)) - i64::from(bottom) + 1;
        let elapsed = (now - start).max(0.0);
        let steps = if self.step_seconds > 0.0 { (elapsed / self.step_seconds).floor() as i64 } else { 0 };
        (i64::from(bottom) + steps % count) as i32
    }
}
