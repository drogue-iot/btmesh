use btmesh_common::SeqZero;
use core::cell::Cell;
use core::future::pending;
use embassy_time::{Instant, Timer};

#[derive(Copy, Clone)]
pub enum WatchdogEvent {
    LinkOpenTimeout,
    OutboundExpiration(SeqZero),
    InboundExpiration(SeqZero),
}

#[derive(Default)]
pub struct Watchdog {
    link_opening_timeout: Cell<Option<(Instant, WatchdogEvent)>>,
    outbound_expiration: Cell<Option<(Instant, WatchdogEvent)>>,
    inbound_expiration: Cell<Option<(Instant, WatchdogEvent)>>,
}

impl Watchdog {
    fn earliest(
        left: Option<(Instant, WatchdogEvent)>,
        right: Option<(Instant, WatchdogEvent)>,
    ) -> Option<(Instant, WatchdogEvent)> {
        match (left, right) {
            (None, Some(_)) => right,
            (Some(_), None) => left,
            (Some(inner_left), Some(inner_right)) if inner_left.0 < inner_right.0 => left,
            (Some(inner_left), Some(inner_right)) if inner_right.0 < inner_left.0 => right,
            _ => None,
        }
    }

    #[allow(clippy::let_unit_value)]
    pub async fn next(&self) -> Option<Expiration<'_>> {
        let next = Self::earliest(
            Self::earliest(None, self.link_opening_timeout.get()),
            Self::earliest(
                self.outbound_expiration.get(),
                self.inbound_expiration.get(),
            ),
        );

        if let Some(next) = next {
            Timer::at(next.0).await;
            Some(Expiration::new(self, next.1))
        } else {
            let _: () = pending().await;
            None
        }
    }

    pub fn link_opening_timeout(&self, expiration: Instant) {
        self.link_opening_timeout
            .replace(Some((expiration, WatchdogEvent::LinkOpenTimeout)));
    }

    pub fn clear_link_open_timeout(&self) {
        self.link_opening_timeout.take();
    }

    pub fn outbound_expiration(&self, expiration: (Instant, SeqZero)) {
        if let Some(current) = self.outbound_expiration.get() {
            if current.0 < expiration.0 {
                return;
            }
            self.outbound_expiration.replace(Some((
                expiration.0,
                WatchdogEvent::OutboundExpiration(expiration.1),
            )));
        } else {
            self.outbound_expiration.replace(Some((
                expiration.0,
                WatchdogEvent::OutboundExpiration(expiration.1),
            )));
        }
    }

    pub fn clear_outbound_expiration(&self, seq_zero: SeqZero) {
        if let Some((_, WatchdogEvent::OutboundExpiration(current))) =
            self.outbound_expiration.get()
        {
            if current == seq_zero {
                self.outbound_expiration.take();
            }
        }
    }

    pub fn clear_inbound_expiration(&self, seq_zero: SeqZero) {
        if let Some((_, WatchdogEvent::InboundExpiration(current))) = self.inbound_expiration.get()
        {
            if current == seq_zero {
                self.inbound_expiration.take();
            }
        }
    }

    pub fn inbound_expiration(&self, expiration: (Instant, SeqZero)) {
        if let Some(current) = self.inbound_expiration.get() {
            if current.0 < expiration.0 {
                return;
            }
            self.inbound_expiration.replace(Some((
                expiration.0,
                WatchdogEvent::InboundExpiration(expiration.1),
            )));
        } else {
            self.inbound_expiration.replace(Some((
                expiration.0,
                WatchdogEvent::InboundExpiration(expiration.1),
            )));
        }
    }
}

pub struct Expiration<'w> {
    watchdog: &'w Watchdog,
    event: WatchdogEvent,
}

impl<'w> Expiration<'w> {
    fn new(watchdog: &'w Watchdog, event: WatchdogEvent) -> Self {
        Self { watchdog, event }
    }

    pub fn take(self) -> WatchdogEvent {
        match self.event {
            WatchdogEvent::LinkOpenTimeout => {
                self.watchdog.clear_link_open_timeout();
            }
            WatchdogEvent::OutboundExpiration(seq_zero) => {
                self.watchdog.clear_outbound_expiration(seq_zero);
            }
            WatchdogEvent::InboundExpiration(seq_zero) => {
                self.watchdog.clear_inbound_expiration(seq_zero);
            }
        }

        self.event
    }
}
