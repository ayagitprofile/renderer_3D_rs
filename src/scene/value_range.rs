#[derive(Clone, Copy)]
pub struct ValueRange {
    value: f32,
    min: f32,
    max: f32,
}

impl ValueRange {
    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn new(value: f32, min: f32, max: f32) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
        }
    }
}
