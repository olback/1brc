type Float = f32; // Performance: f32 is faster than f64?

#[derive(Debug, Clone)]
pub struct Values {
    pub min: Float,
    pub max: Float,
    pub sum: Float,
    pub count: usize,
}

impl Values {
    pub fn new(value: Float) -> Self {
        Self {
            min: value,
            max: value,
            sum: value,
            count: 1,
        }
    }

    pub fn add(&mut self, value: Float) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum += other.sum;
        self.count += other.count;
    }

    pub fn mean(&self) -> Float {
        self.sum / self.count as Float
    }
}
