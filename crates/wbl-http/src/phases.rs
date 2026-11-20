//! DNS and connect times of the connection a send opens (D-H20).
//!
//! Clients are shared, so a resolver or connector cannot know which send it works for. Each send
//! runs inside [`recorded`], which puts a recorder in a task-local; hyper runs the connect (and
//! its DNS lookup) inside the send's own future, so both find it there. A send that reuses a
//! pooled connection records nothing.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// What one send spent opening connections (all of them, when it followed redirects).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Phases {
    pub dns: Option<Duration>,
    /// Connecting, DNS included.
    pub connect: Option<Duration>,
}

impl Phases {
    /// Connecting without the DNS part: TCP and TLS.
    pub fn connect_only(self) -> Option<Duration> {
        self.connect
            .map(|c| c.saturating_sub(self.dns.unwrap_or_default()))
    }
}

type Recorder = Arc<Mutex<Phases>>;

tokio::task_local! {
    static RECORDER: Recorder;
}

/// Runs `future` (a send) and returns what it spent on DNS and connecting.
pub(crate) async fn recorded<F: Future>(future: F) -> (F::Output, Phases) {
    let recorder = Recorder::default();
    let output = RECORDER.scope(Arc::clone(&recorder), future).await;
    // A poisoned lock only loses the breakdown, never the response.
    let phases = recorder.lock().map(|p| *p).unwrap_or_default();
    (output, phases)
}

fn current() -> Option<Recorder> {
    RECORDER.try_with(Arc::clone).ok()
}

fn plus(total: Option<Duration>, more: Duration) -> Option<Duration> {
    Some(total.unwrap_or_default() + more)
}

fn record(recorder: Option<&Recorder>, change: impl FnOnce(Phases) -> Phases) {
    if let Some(Ok(mut phases)) = recorder.map(|r| r.lock()) {
        *phases = change(*phases);
    }
}

/// The system resolver (`getaddrinfo` on a blocking thread, as reqwest's default), timed.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TimedResolver;

impl Resolve for TimedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let recorder = current();
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let started = Instant::now();
            let found = tokio::net::lookup_host((host.as_str(), 0)).await?;
            let addrs: Vec<_> = found.collect();
            record(recorder.as_ref(), |p| Phases {
                dns: plus(p.dns, started.elapsed()),
                ..p
            });
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// A connector layer that times each connection it opens.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TimedConnect;

impl<S> tower_layer::Layer<S> for TimedConnect {
    type Service = Timed<S>;

    fn layer(&self, inner: S) -> Timed<S> {
        Timed(inner)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Timed<S>(S);

type Boxed<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

impl<S, R> tower_service::Service<R> for Timed<S>
where
    S: tower_service::Service<R>,
    S::Future: Send + 'static,
    S::Response: 'static,
    S::Error: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Boxed<S::Response, S::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, target: R) -> Self::Future {
        let recorder = current();
        let started = Instant::now();
        let connecting = self.0.call(target);
        Box::pin(async move {
            let result = connecting.await;
            if result.is_ok() {
                record(recorder.as_ref(), |p| Phases {
                    connect: plus(p.connect, started.elapsed()),
                    ..p
                });
            }
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_only_leaves_out_the_dns_part() {
        let phases = Phases {
            dns: Some(Duration::from_millis(5)),
            connect: Some(Duration::from_millis(12)),
        };
        assert_eq!(phases.connect_only(), Some(Duration::from_millis(7)));
        assert_eq!(Phases::default().connect_only(), None);
    }

    #[tokio::test]
    async fn nothing_is_recorded_outside_a_send() {
        assert!(current().is_none());
        let ((), phases) = recorded(async {
            record(current().as_ref(), |p| Phases {
                dns: plus(p.dns, Duration::from_millis(3)),
                ..p
            });
        })
        .await;
        assert_eq!(phases.dns, Some(Duration::from_millis(3)));
    }
}
