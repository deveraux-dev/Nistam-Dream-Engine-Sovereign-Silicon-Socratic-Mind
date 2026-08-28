//! Vixitic — a deterministic tick-reactor async runtime.
//!
//! `async` in Rust is a compiler state machine plus a `Waker` contract: "am I
//! done? no? call this when I should re-check." Tokio answers "who calls the
//! waker" with an OS reactor (epoll/IOCP). Vixitic answers it with *your* integer
//! clock: a task wakes on the exact tick your simulation says it should, never
//! on a socket, a thread scheduler, or a wall clock.
//!
//! That makes every resume a pure function of `(condition, event schedule)`,
//! which is what lockstep netcode, rollback, emulators and replayable
//! simulations actually need. Two runs of the same schedule produce a
//! byte-identical resume log — see `tests/integration.rs`.
//!
//! Two primitives, nothing else:
//!
//! * [`Runtime`] — a loop draining a run-queue of tasks. Driven whole
//!   ([`Runtime::block_on`]) or one tick at a time ([`Runtime::step`], where an
//!   external metronome IS the clock).
//! * [`Reactor`] — parks `(Cond, Waker)` pairs; [`Reactor::advance`] returns the
//!   wakers whose condition the tick satisfies. Your engine feeds it.
//!
//! ```
//! use vixitic::{sleep_ticks, spawn, Runtime};
//!
//! let rt = Runtime::new();
//! let woke = rt.block_on(
//!     async {
//!         spawn(async { sleep_ticks(3).await; });
//!         sleep_ticks(5).await
//!     },
//!     |_tick| Vec::new(), // no external events this run
//!     10,                 // hang-guard: panic rather than spin forever
//! );
//! assert_eq!(woke, 5);
//! ```
//!
//! Zero dependencies, `std` only.

#![deny(missing_docs)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

type BoxFut = Pin<Box<dyn Future<Output = ()> + Send>>;

// ── Task + Waker ────────────────────────────────────────────────────────────
// A Task is a parked future + the run-queue it re-enqueues itself onto. The
// Waker is a hand-rolled RawWaker vtable over `Arc<Task>`: `wake` == `schedule`.

struct Task {
    fut: Mutex<Option<BoxFut>>,
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    queued: AtomicBool,
}

impl Task {
    /// Push onto the run-queue unless already pending there (idempotent wake).
    fn schedule(self: &Arc<Self>) {
        if !self.queued.swap(true, Ordering::AcqRel) {
            self.queue.lock().unwrap().push_back(self.clone());
        }
    }
}

static VT: RawWakerVTable = RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw);

fn raw(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VT)
}

unsafe fn clone_raw(p: *const ()) -> RawWaker {
    let arc = Arc::from_raw(p as *const Task);
    let cloned = arc.clone();
    std::mem::forget(arc); // don't drop the original refcount
    raw(Arc::into_raw(cloned) as *const ())
}
unsafe fn wake_raw(p: *const ()) {
    // consumes one refcount
    let arc = Arc::from_raw(p as *const Task);
    arc.schedule();
}
unsafe fn wake_by_ref_raw(p: *const ()) {
    let arc = Arc::from_raw(p as *const Task);
    arc.schedule();
    std::mem::forget(arc); // borrow: keep the refcount
}
unsafe fn drop_raw(p: *const ()) {
    drop(Arc::from_raw(p as *const Task));
}

fn waker_of(task: Arc<Task>) -> Waker {
    unsafe { Waker::from_raw(raw(Arc::into_raw(task) as *const ())) }
}

// ── Reactor ─────────────────────────────────────────────────────────────────
// The OS is fired. Readiness is an integer tick and a set of fired event ids.

/// What a parked task is waiting for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cond {
    /// Wake once the clock reaches or passes this absolute tick
    /// (what [`sleep_ticks`] parks on).
    AtTick(u64),
    /// Wake when the engine reports this event id for the current tick
    /// (what [`await_event`] parks on). The id is yours: a collision cell, an
    /// entity handle, a hashed name — Vixitic never interprets it.
    Event(u64),
}

/// A oneshot latch shared between an [`Await`] future and the reactor.
struct Signal {
    inner: Mutex<SignalInner>,
}
struct SignalInner {
    fired: Option<u64>, // Some(tick) once the condition was met
    waker: Option<Waker>,
}
impl Signal {
    fn new() -> Arc<Self> {
        Arc::new(Signal {
            inner: Mutex::new(SignalInner {
                fired: None,
                waker: None,
            }),
        })
    }
}

struct Parked {
    cond: Cond,
    signal: Arc<Signal>,
}

/// Binds wakers to integer clock events. Single-writer, driven by your engine.
pub struct Reactor {
    tick: u64,
    parked: Vec<Parked>,
}

impl Reactor {
    fn new() -> Self {
        Reactor {
            tick: 0,
            parked: Vec::new(),
        }
    }

    /// Advance the clock to `tick` with the event ids the engine fired this
    /// tick. Latches every parked signal whose condition is now satisfied — in
    /// stable registration order, which is the determinism guard — and returns
    /// their wakers **unwoken**.
    ///
    /// The caller wakes them *after* dropping the reactor lock, so
    /// `wake -> schedule` never runs while the reactor mutex is held. That is
    /// what kills the lock-order inversion (`advance` holds the reactor;
    /// `Await::poll` takes signal-then-reactor) and makes the reactor safe to
    /// drive from a thread other than the one that parked the tasks.
    #[must_use = "the returned wakers must be woken after the reactor lock is dropped"]
    pub fn advance(&mut self, tick: u64, events: &[u64]) -> Vec<Waker> {
        self.tick = tick;
        let mut survivors = Vec::with_capacity(self.parked.len());
        let mut wakers = Vec::new();
        for p in self.parked.drain(..) {
            let hit = match p.cond {
                Cond::AtTick(t) => tick >= t,
                Cond::Event(id) => events.contains(&id),
            };
            if hit {
                let waker = {
                    let mut g = p.signal.inner.lock().unwrap();
                    g.fired = Some(tick);
                    g.waker.take()
                };
                if let Some(w) = waker {
                    wakers.push(w);
                }
            } else {
                survivors.push(p);
            }
        }
        self.parked = survivors;
        wakers
    }

    /// The tick this reactor last advanced to.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ── Per-thread execution context ────────────────────────────────────────────
// Set while a Runtime is driving on this thread. `spawn`/`await_event`/
// `sleep_ticks` read it. Single-threaded executor, so a RefCell is enough.

struct Ctx {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    reactor: Arc<Mutex<Reactor>>,
}

thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}

fn with_ctx<R>(f: impl FnOnce(&Ctx) -> R) -> R {
    CTX.with(|c| {
        let b = c.borrow();
        let ctx = b
            .as_ref()
            .expect("vixitic: called outside Runtime::block_on / Runtime::step");
        f(ctx)
    })
}

// ── Await future ────────────────────────────────────────────────────────────

/// The future returned by [`await_event`] / [`sleep_ticks`]. Resolves to the
/// tick it was woken on.
pub struct Await {
    cond: Cond,
    signal: Arc<Signal>,
    reactor: Arc<Mutex<Reactor>>,
    registered: bool,
}

impl Future for Await {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        let me = self.get_mut();
        let mut g = me.signal.inner.lock().unwrap();
        if let Some(tick) = g.fired {
            return Poll::Ready(tick);
        }
        g.waker = Some(cx.waker().clone());
        drop(g);
        if !me.registered {
            me.registered = true;
            me.reactor.lock().unwrap().parked.push(Parked {
                cond: me.cond,
                signal: me.signal.clone(),
            });
        }
        Poll::Pending
    }
}

fn park(cond: Cond) -> Await {
    with_ctx(|ctx| Await {
        cond,
        signal: Signal::new(),
        reactor: ctx.reactor.clone(),
        registered: false,
    })
}

/// Park until the engine fires event `id`. Resumes on the exact integer tick
/// the engine reported it.
///
/// # Panics
/// Panics if called outside a driving [`Runtime`].
pub fn await_event(id: u64) -> Await {
    park(Cond::Event(id))
}

/// Park for `n` ticks of the integer clock, relative to now.
///
/// # Panics
/// Panics if called outside a driving [`Runtime`].
pub fn sleep_ticks(n: u64) -> Await {
    let now = with_ctx(|ctx| ctx.reactor.lock().unwrap().tick);
    park(Cond::AtTick(now + n))
}

// ── Spawn ───────────────────────────────────────────────────────────────────

fn make_task(fut: BoxFut, queue: &Arc<Mutex<VecDeque<Arc<Task>>>>) -> Arc<Task> {
    let t = Arc::new(Task {
        fut: Mutex::new(Some(fut)),
        queue: queue.clone(),
        queued: AtomicBool::new(false),
    });
    t.schedule();
    t
}

/// Spawn a child task onto the current runtime. Fire-and-forget: the handle is
/// not returned, so a child that outlives the root is simply dropped.
///
/// # Panics
/// Panics if called outside a driving [`Runtime`].
pub fn spawn<F: Future<Output = ()> + Send + 'static>(fut: F) {
    with_ctx(|ctx| {
        make_task(Box::pin(fut), &ctx.queue);
    });
}

// ── Runtime / executor ──────────────────────────────────────────────────────

/// The Vixio runtime: drain the run-queue, advance the integer clock, repeat.
pub struct Runtime {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    reactor: Arc<Mutex<Reactor>>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// A runtime with an empty queue and its clock at tick 0.
    #[must_use]
    pub fn new() -> Self {
        Runtime {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            reactor: Arc::new(Mutex::new(Reactor::new())),
        }
    }

    /// Drive `root` to completion and return its output.
    ///
    /// `engine(tick) -> fired event ids` is your readiness oracle — the one
    /// place the outside world enters. `max_tick` is a loud hang-guard: if the
    /// root has not completed by then this panics rather than spinning forever,
    /// because a silently stalled deterministic runtime is the worst outcome.
    ///
    /// # Panics
    /// Panics if `root` has not completed by `max_tick`.
    pub fn block_on<F, T, E>(&self, root: F, mut engine: E, max_tick: u64) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        E: FnMut(u64) -> Vec<u64>,
    {
        let out: Arc<Mutex<Option<T>>> = Arc::new(Mutex::new(None));
        let out2 = out.clone();
        let root_fut: BoxFut = Box::pin(async move {
            let v = root.await;
            *out2.lock().unwrap() = Some(v);
        });

        CTX.with(|c| {
            *c.borrow_mut() = Some(Ctx {
                queue: self.queue.clone(),
                reactor: self.reactor.clone(),
            });
        });
        make_task(root_fut, &self.queue);

        let mut tick = 0u64;
        loop {
            // 1. Drain: poll every ready task until the queue is empty.
            drain(&self.queue);
            // 2. Done?
            if out.lock().unwrap().is_some() {
                break;
            }
            // 3. Advance the integer clock and let the engine fire events.
            if tick >= max_tick {
                CTX.with(|c| *c.borrow_mut() = None);
                panic!(
                    "vixitic: root future stalled past max_tick={max_tick} — a Cond never fired (deadlocked reactor)"
                );
            }
            tick += 1;
            let events = engine(tick);
            // Latch fired wakers under the lock, wake them after releasing it.
            let wakers = self.reactor.lock().unwrap().advance(tick, &events);
            for w in wakers {
                w.wake();
            }
        }

        CTX.with(|c| *c.borrow_mut() = None);
        let result = out.lock().unwrap().take().expect("root completed");
        result
    }

    /// Seed a task onto this runtime without driving it — the stepped twin of
    /// [`spawn`]. First polled on the next [`step`](Runtime::step) or
    /// [`block_on`](Runtime::block_on) drain.
    pub fn spawn_on<F: Future<Output = ()> + Send + 'static>(&self, fut: F) {
        make_task(Box::pin(fut), &self.queue);
    }

    /// One integer tick — the stepped twin of [`block_on`](Runtime::block_on).
    /// Drain ready tasks, advance the persisted reactor clock by one, wake what
    /// fired, then drain again so same-tick completions are visible before this
    /// returns. Returns the tick just executed.
    ///
    /// Queue, clock and parked conditions survive across calls, so a task
    /// parked on `sleep_ticks(n)` spans n calls and your own frame loop can BE
    /// the clock. Never mix with `block_on` on one runtime: `block_on`'s local
    /// tick restarts at 0 and would rewind the schedule.
    pub fn step<E: FnMut(u64) -> Vec<u64>>(&self, mut engine: E) -> u64 {
        CTX.with(|c| {
            *c.borrow_mut() = Some(Ctx {
                queue: self.queue.clone(),
                reactor: self.reactor.clone(),
            });
        });
        drain(&self.queue);
        let tick = self.reactor.lock().unwrap().tick + 1;
        let events = engine(tick);
        let wakers = self.reactor.lock().unwrap().advance(tick, &events);
        for w in wakers {
            w.wake();
        }
        drain(&self.queue);
        CTX.with(|c| *c.borrow_mut() = None);
        tick
    }

    /// The tick this runtime's clock currently stands on.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.reactor.lock().unwrap().tick
    }
}

/// Drain the run-queue: poll every ready task until it is empty.
fn drain(queue: &Arc<Mutex<VecDeque<Arc<Task>>>>) {
    loop {
        let next = queue.lock().unwrap().pop_front();
        match next {
            Some(task) => poll_task(task),
            None => break,
        }
    }
}

fn poll_task(task: Arc<Task>) {
    // Clear the queued flag BEFORE polling so a wake during poll re-enqueues.
    task.queued.store(false, Ordering::Release);
    let mut slot = task.fut.lock().unwrap();
    if let Some(mut fut) = slot.take() {
        let waker = waker_of(task.clone());
        let mut cx = Context::from_waker(&waker);
        match fut.as_mut().poll(&mut cx) {
            Poll::Pending => *slot = Some(fut),
            Poll::Ready(()) => {}
        }
    }
}
