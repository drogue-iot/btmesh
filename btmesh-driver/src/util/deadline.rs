use core::cell::Cell;
use embassy::time::{Duration, Instant};

pub struct Deadline {
    every: Duration,
    next: Cell<Instant>,
}

impl Deadline {

    pub fn new(every: Duration) -> Self {
        Self {
            every,
            next: Cell::new(Instant::now() + every),
        }
    }

    pub fn next(&self) -> Instant {
        let now = Instant::now();

        if self.next.get() <= now {
            self.next.replace(now + self.every);
        }

        self.next.get()
    }

}
