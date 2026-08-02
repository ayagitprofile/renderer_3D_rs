pub struct ScopedTimer {
    start: std::time::Instant,
    description: std::string::String,
}

impl ScopedTimer {
    pub fn new(description: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            description: std::string::String::from(description),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let now = std::time::Instant::now();
        let duration = now.duration_since(self.start);
        let elapsed_time = duration.as_millis();

        let displayed_time = if elapsed_time == 0 {
            "< 0".to_string()
        } else {
            elapsed_time.to_string()
        };

        println!("[Timer] {} took: {} ms", self.description, displayed_time);
    }
}
