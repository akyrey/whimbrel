//! Requests in flight, by the id the frontend gave them, so `request_cancel` can stop one.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio_util::sync::CancellationToken;

use crate::error::AppError;

/// One running request: its token, and which start it came from.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub token: CancellationToken,
    generation: u64,
}

#[derive(Default)]
struct Entries {
    next: u64,
    running: HashMap<String, Ticket>,
}

#[derive(Default)]
pub struct InFlight(Mutex<Entries>);

fn poisoned() -> AppError {
    AppError::internal("The request registry is unusable. Restart Whimbrel and try again.")
}

impl InFlight {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Entries>, AppError> {
        self.0.lock().map_err(|_| poisoned())
    }

    /// Registers a request under `id`. One already running under `id` is cancelled: a tab sends
    /// one request at a time.
    pub fn start(&self, id: &str) -> Result<Ticket, AppError> {
        let mut entries = self.lock()?;
        entries.next += 1;
        let ticket = Ticket {
            token: CancellationToken::new(),
            generation: entries.next,
        };
        if let Some(previous) = entries.running.insert(id.to_owned(), ticket.clone()) {
            previous.token.cancel();
        }
        Ok(ticket)
    }

    /// Forgets `id`, unless a newer request has taken its place.
    pub fn finish(&self, id: &str, ticket: &Ticket) -> Result<(), AppError> {
        let mut entries = self.lock()?;
        if entries
            .running
            .get(id)
            .is_some_and(|current| current.generation == ticket.generation)
        {
            entries.running.remove(id);
        }
        Ok(())
    }

    /// Cancels `id`; `false` when nothing is running under it (it already finished).
    pub fn cancel(&self, id: &str) -> Result<bool, AppError> {
        let ticket = self.lock()?.running.remove(id);
        Ok(ticket.map(|t| t.token.cancel()).is_some())
    }

    pub fn len(&self) -> usize {
        self.lock().map(|e| e.running.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn cancel_stops_a_running_request_once() {
        let in_flight = InFlight::default();
        let ticket = in_flight.start("tab-1").unwrap();

        assert!(in_flight.cancel("tab-1").unwrap());
        assert!(ticket.token.is_cancelled());
        assert!(!in_flight.cancel("tab-1").unwrap(), "already gone");
    }

    #[test]
    fn a_new_request_on_the_same_tab_cancels_the_old_one() {
        let in_flight = InFlight::default();
        let first = in_flight.start("tab-1").unwrap();
        let second = in_flight.start("tab-1").unwrap();

        in_flight.finish("tab-1", &first).unwrap();

        assert!(first.token.is_cancelled());
        assert!(!second.token.is_cancelled());
        assert_eq!(
            in_flight.len(),
            1,
            "the old one's finish leaves the new one"
        );
        in_flight.finish("tab-1", &second).unwrap();
        assert!(in_flight.is_empty());
    }

    #[test]
    fn tabs_are_independent() {
        let in_flight = InFlight::default();
        let one = in_flight.start("tab-1").unwrap();
        let two = in_flight.start("tab-2").unwrap();

        in_flight.cancel("tab-2").unwrap();

        assert!(!one.token.is_cancelled());
        assert!(two.token.is_cancelled());
    }
}
