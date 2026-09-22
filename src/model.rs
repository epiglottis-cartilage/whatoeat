use serde::{Deserialize, Serialize};

pub const POLICY: &str = "logistic-b-v1";
pub const TARGET: f64 = 0.7;
pub const MIN_SLOPE: f64 = 0.05;

/// Two learned parameters. A fixed prior keeps sparse feedback well behaved.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Model {
    pub a: f64,
    pub b: f64,
    pub samples: u64,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            a: (TARGET / (1.0 - TARGET)).ln() - 2.0 * 8_f64.ln(),
            b: 2.0,
            samples: 0,
        }
    }
}

impl Model {
    pub fn predict(&self, days: f64) -> f64 {
        let z = self.a + self.b * days.max(0.0).ln_1p();
        if z >= 0.0 {
            1.0 / (1.0 + (-z).exp())
        } else {
            let e = z.exp();
            e / (1.0 + e)
        }
    }

    pub fn update(&mut self, days: f64, eat: bool) {
        let prior = Self::default();
        let x = days.max(0.0).ln_1p();
        let error = self.predict(days) - f64::from(eat);
        let rate = 0.08 / (1.0 + self.samples as f64 / 8.0).powf(0.65);
        // Both gradients use the old state; project onto convex parameter bounds.
        let a = self.a - rate * (error + 0.002 * (self.a - prior.a));
        let b = self.b - rate * (error * x + 0.002 * (self.b - prior.b));
        self.a = a.clamp(-30.0, 30.0);
        self.b = b.clamp(MIN_SLOPE, 20.0);
        self.samples += 1;
    }

    /// None means outside our display range, not that a 180-day cycle was learned.
    pub fn interval(&self) -> Option<f64> {
        let exponent = ((TARGET / (1.0 - TARGET)).ln() - self.a) / self.b;
        let days = exponent.exp_m1().max(0.0);
        (days.is_finite() && days <= 180.0).then_some(days)
    }
}
