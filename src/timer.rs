use std::time::Duration;

pub struct Timer {
    start: std::time::Instant,
    description: std::string::String,
}

pub struct ScopedTimer {
    timer: Timer,
}

impl Timer {
    pub fn start(description: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            description: std::string::String::from(description),
        }
    }

    pub fn elapsed(&self) -> Duration {
        std::time::Instant::now().duration_since(self.start)
    }

    pub fn reset(&mut self) -> Duration {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.start);
        self.start = now;
        elapsed
    }

    pub fn print_elapsed(&self) {
        let elapsed_time = self.elapsed().as_millis();

        let displayed_time = if elapsed_time == 0 {
            "< 0".to_string()
        } else {
            elapsed_time.to_string()
        };

        println!("[Timer] {} took: {} ms", self.description, displayed_time);
    }
}

impl ScopedTimer {
    pub fn start(description: &str) -> Self {
        Self {
            timer: Timer {
                start: std::time::Instant::now(),
                description: String::from(description),
            },
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        self.timer.print_elapsed();
    }
}
