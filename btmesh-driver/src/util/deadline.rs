use core::cell::Cell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use embassy_time::{Duration, Instant, Timer};

pub struct Deadline {
    every: Duration,
    next: Cell<Instant>,
}

impl Deadline {
    pub fn new(every: Duration, immediate: bool) -> Self {
        Self {
            every,
            next: Cell::new(
                Instant::now()
                    + if immediate {
                        Duration::default()
                    } else {
                        every
                    },
            ),
        }
    }

    pub fn next(&self) -> DeadlineFuture<'_> {
        let now = Instant::now();
        if self.next.get() <= now {
            DeadlineFuture::new(self, now)
        } else {
            DeadlineFuture::new(self, self.next.get())
        }
    }

    fn advance(&self) {
        self.next.replace(Instant::now() + self.every);
    }
}

pub struct DeadlineFuture<'d> {
    deadline: &'d Deadline,
    timer: Timer,
}

impl<'d> DeadlineFuture<'d> {
    fn new(deadline: &'d Deadline, instant: Instant) -> Self {
        Self {
            deadline,
            timer: Timer::at(instant),
        }
    }
}

impl<'d> Future for DeadlineFuture<'d> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = Pin::new(&mut self.timer).poll(cx);

        if result.is_ready() {
            self.deadline.advance();
        }

        result
    }
}
