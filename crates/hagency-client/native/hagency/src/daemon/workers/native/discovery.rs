use super::*;
use std::sync::Mutex;

// The inbox holds one original demand reference per protocol. It carries no
// launch proof. The consumer reads its current status before using it.
pub(super) struct Offer {
    result: Result<(String, String), NativeError>,
    handled: tokio::sync::oneshot::Sender<()>,
    observed: std::time::Instant,
}

pub(super) type Inbox = Mutex<Option<Offer>>;

pub(super) async fn poll(
    budget: bool,
    inbox: &Inbox,
    executors: &tokio::sync::Mutex<Option<Arc<CurrentSharedExecutor>>>,
    admission: &Admission,
    control: &Control,
    live: &(impl Fn() -> bool + Sync),
) {
    let step = if budget {
        Step::BudgetDemand
    } else {
        Step::IntakeDemand
    };
    if (budget && !cfg!(feature = "shared-budget-execution"))
        || (!budget && !cfg!(feature = "shared-intake"))
    {
        return;
    }
    discover_loop(inbox, admission, control, live, step, |after| async move {
        let executor = executors.lock().await.clone();
        match executor {
            Some(executor) => executor.discover_for_worker(budget, after).await,
            None => Ok((after, None)),
        }
    })
    .await;
}

async fn discover_loop<Fut>(
    inbox: &Inbox,
    admission: &Admission,
    control: &Control,
    live: &(impl Fn() -> bool + Sync),
    step: Step,
    mut discover: impl FnMut(Option<String>) -> Fut + Send,
) where
    Fut: Future<Output = Result<(Option<String>, Option<(String, String)>), NativeError>> + Send,
{
    // Notify registers this future at creation for notify_waiters. Check the
    // retired bit afterwards as retirement can precede this producer.
    let retiring = control.retiring.notified();
    tokio::pin!(retiring);
    let mut after = None;
    while live() {
        if let Some(guard) = admission.enter() {
            let started = std::time::Instant::now();
            let found = match discover(after.clone()).await {
                Ok((cursor, offer)) => {
                    after = cursor;
                    offer.map(Ok)
                }
                Err(error) => Some(Err(error)),
            };
            drop(guard);
            tracing::debug!(
                ?step,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "Owner daemon demand discovery finished"
            );
            if let Some(result) = found
                && live()
            {
                let (handled, mut ack) = tokio::sync::oneshot::channel();
                *inbox.lock().unwrap_or_else(|p| p.into_inner()) = Some(Offer {
                    result,
                    handled,
                    observed: std::time::Instant::now(),
                });
                control.wake.notify_one();
                // Leave the server queue authoritative while admission is paused
                // or this worker is retired. Never collect a backlog of tickets.
                while live() {
                    tokio::select! {
                        _ = &mut ack => break,
                        _ = &mut retiring => return,
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                    }
                }
            }
        }
        if live() {
            tokio::select! {
                _ = &mut retiring => return,
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
        }
    }
}

pub(super) struct Consumer<'a> {
    pub executor: &'a CurrentSharedExecutor,
    pub ordinary: &'a Inbox,
    pub budget: &'a Inbox,
}

impl Consumer<'_> {
    fn inbox(&self, step: Step) -> Option<&Inbox> {
        match step {
            Step::IntakeDemand => Some(self.ordinary),
            Step::BudgetDemand => Some(self.budget),
            _ => None,
        }
    }
}

impl PassIo for Consumer<'_> {
    fn ready(&self, step: Step) -> bool {
        self.inbox(step)
            .is_some_and(|inbox| inbox.lock().unwrap_or_else(|p| p.into_inner()).is_some())
    }
    fn enabled(&self, step: Step) -> bool {
        self.executor.enabled(step) && (self.inbox(step).is_none() || self.ready(step))
    }
    fn run(&self, step: Step, after: Option<String>) -> StepFuture<'_> {
        let Some(inbox) = self.inbox(step) else {
            return self.executor.run(step, after);
        };
        let offer = inbox.lock().unwrap_or_else(|p| p.into_inner()).take();
        Box::pin(async move {
            let Some(Offer {
                result,
                handled,
                observed,
            }) = offer
            else {
                return Ok(Advance::default());
            };
            let (id, digest) = result?;
            tracing::debug!(
                ?step,
                handoff_wait_ms = observed.elapsed().as_millis() as u64,
                "Owner daemon picked up a discovered demand"
            );
            let outcome = self
                .executor
                .consume_for_worker(step == Step::BudgetDemand, &id, &digest)
                .await;
            let _ = handled.send(());
            Ok(Advance {
                cursor: None,
                note: demand_note(&outcome),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retiring_wakes_both_idle_discovery_lanes() {
        let worker = Worker::default();
        let admission = Admission::default();
        admission.open();
        let ordinary = Inbox::default();
        let budget = Inbox::default();
        let live = || !worker.retired();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let empty = |cursor| {
            calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok((cursor, None)))
        };
        let retire = async {
            while calls.load(Ordering::SeqCst) < 2 {
                tokio::task::yield_now().await;
            }
            worker.retire();
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(
                discover_loop(
                    &ordinary,
                    &admission,
                    &worker.control,
                    &live,
                    Step::IntakeDemand,
                    empty
                ),
                discover_loop(
                    &budget,
                    &admission,
                    &worker.control,
                    &live,
                    Step::BudgetDemand,
                    empty
                ),
                retire
            );
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn discovery_runs_while_maintenance_is_waiting_and_keeps_one_unclaimed_reference() {
        let inbox = Inbox::default();
        let admission = Admission::default();
        admission.open();
        let control = Control::default();
        let live = || !control.retired.load(Ordering::Acquire);
        let held = tokio::sync::Notify::new();
        let entered = tokio::sync::Notify::new();
        let (observed_tx, observed_rx) = tokio::sync::oneshot::channel();
        let maintenance = async {
            entered.notify_one();
            held.notified().await;
            observed_tx.send(()).unwrap();
        };
        let poll = async {
            entered.notified().await;
            discover_loop(
                &inbox,
                &admission,
                &control,
                &live,
                Step::IntakeDemand,
                |cursor| async move {
                    assert!(cursor.is_none());
                    Ok((
                        Some("assignment".into()),
                        Some(("original".into(), "digest".into())),
                    ))
                },
            )
            .await;
        };
        let observe = async {
            control.wake.notified().await;
            let offer = inbox.lock().unwrap().take().unwrap();
            assert_eq!(offer.result.unwrap().0, "original");
            control.retired.store(true, Ordering::Release);
            let _ = offer.handled.send(());
            held.notify_one();
            observed_rx.await.unwrap();
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(maintenance, poll, observe);
        })
        .await
        .unwrap();
    }
}
