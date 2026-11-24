use super::*;
use any_spawner::Executor;
use futures::channel::oneshot;
use reactive_graph::{
    owner::on_cleanup,
    signal::RwSignal,
    traits::{Get, Set, Track},
    wrappers::read::Signal,
};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tachys::{
    reactive_graph::Suspend,
    renderer::types::{Element, Node},
};

struct DropProbe(Arc<AtomicUsize>);

struct DropProbeState(Arc<AtomicUsize>);

impl Render for DropProbe {
    type State = DropProbeState;

    fn build(self) -> Self::State {
        DropProbeState(self.0)
    }

    fn rebuild(self, state: &mut Self::State) {
        state.0 = self.0;
    }
}

impl Drop for DropProbeState {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl Mountable for DropProbeState {
    fn unmount(&mut self) {}

    fn mount(&mut self, _parent: &Element, _marker: Option<&Node>) {}

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<Element> {
        Vec::new()
    }
}

fn boundary<const TRANSITION: bool, Chil>(
    children: Chil,
    drops: Arc<AtomicUsize>,
) -> SuspenseBoundary<TRANSITION, DropProbe, Chil> {
    SuspenseBoundary {
        id: Default::default(),
        none_pending: ArcMemo::new(|_| true),
        fallback: DropProbe(drops),
        children,
        error_boundary_parent: None,
        has_tasks: Arc::new(|| false),
        strict: false,
    }
}

fn assert_synchronous_disposal<const TRANSITION: bool>() {
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let drops = Arc::new(AtomicUsize::new(0));
        let state = boundary::<TRANSITION, _>(
            DropProbe(Arc::clone(&drops)),
            Arc::clone(&drops),
        )
        .build();
        Executor::poll_local();

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(state);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            2,
            "children and fallback must be dropped before the executor runs"
        );
        Executor::poll_local();
    });
}

#[test]
fn suspense_drops_children_and_fallback_synchronously() {
    assert_synchronous_disposal::<false>();
}

#[test]
fn transition_drops_children_and_fallback_synchronously() {
    assert_synchronous_disposal::<true>();
}

fn assert_queued_child_cancellation<const TRANSITION: bool>() {
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let generation = RwSignal::new(0);
        let child_signal = Arc::new(Mutex::new(None));
        let reads = Arc::new(Mutex::new(Vec::new()));
        let drops = Arc::new(AtomicUsize::new(0));
        let state = {
            let child_signal = Arc::clone(&child_signal);
            let reads = Arc::clone(&reads);
            let drops = Arc::clone(&drops);
            move || {
                let signal = RwSignal::new(generation.get());
                *child_signal.lock().unwrap() = Some(signal);
                let reads = Arc::clone(&reads);
                let child_drops = Arc::clone(&drops);
                boundary::<TRANSITION, _>(
                    move || {
                        reads.lock().unwrap().push(signal.try_get());
                        DropProbe(Arc::clone(&child_drops))
                    },
                    Arc::clone(&drops),
                )
            }
        }
        .build();
        Executor::poll_local();
        let old_signal = child_signal.lock().unwrap().unwrap();

        generation.set(1);
        old_signal.set(42);
        Executor::poll_local();

        assert_eq!(old_signal.try_get(), None);
        assert_eq!(
            *reads.lock().unwrap(),
            vec![Some(0), Some(1)],
            "a replaced child's queued binding must not read its disposed \
             signal"
        );
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        drop(state);
        Executor::poll_local();
    });
}

#[test]
fn suspense_cancels_queued_children_when_parent_rebuilds() {
    assert_queued_child_cancellation::<false>();
}

#[test]
fn transition_cancels_queued_children_when_parent_rebuilds() {
    assert_queued_child_cancellation::<true>();
}

fn poll_local_repeatedly() {
    use std::thread;
    use std::time::Duration;

    for _ in 0..20 {
        Executor::poll_local();
        thread::sleep(Duration::from_millis(1));
    }
}

type Sender = oneshot::Sender<()>;
type Receiver = oneshot::Receiver<()>;

#[derive(Clone, Default)]
struct Loads(Arc<Mutex<HashMap<i32, (Option<Sender>, Option<Receiver>)>>>);

impl Loads {
    fn block(&self, generation: i32) {
        let (tx, rx) = oneshot::channel();
        self.0
            .lock()
            .unwrap()
            .insert(generation, (Some(tx), Some(rx)));
    }

    fn unblock(&self, generation: i32) {
        let tx = self
            .0
            .lock()
            .unwrap()
            .get_mut(&generation)
            .and_then(|(tx, _)| tx.take())
            .expect("generation was not blocked");
        _ = tx.send(());
    }

    fn load(
        &self,
        generation: i32,
    ) -> impl Future<Output = ()> + Send + 'static {
        let rx = self
            .0
            .lock()
            .unwrap()
            .get_mut(&generation)
            .and_then(|(_, rx)| rx.take());
        async move {
            if let Some(rx) = rx {
                _ = rx.await;
            }
        }
    }
}

#[derive(Clone)]
struct RerunTest {
    generation: RwSignal<i32>,
    active: RwSignal<i32>,
    loads: Loads,
    prop_reads: Arc<Mutex<Vec<(i32, Option<i32>)>>>,
    props: Arc<Mutex<Vec<(i32, Signal<i32>)>>>,
    drops: Arc<AtomicUsize>,
}

impl RerunTest {
    fn new() -> Self {
        Self {
            generation: RwSignal::new(0),
            active: RwSignal::new(0),
            loads: Loads::default(),
            prop_reads: Default::default(),
            props: Default::default(),
            drops: Default::default(),
        }
    }

    fn prop(&self, generation: i32) -> Signal<i32> {
        let prop = Signal::stored(generation);
        self.props.lock().unwrap().push((generation, prop));
        prop
    }

    fn view_reading_prop(
        self,
        generation: i32,
        prop: Signal<i32>,
    ) -> impl FnMut() -> DropProbe + Send + 'static {
        move || {
            self.active.track();
            self.prop_reads
                .lock()
                .unwrap()
                .push((generation, prop.try_get()));
            DropProbe(Arc::clone(&self.drops))
        }
    }

    fn prop_reads(&self) -> Vec<(i32, Option<i32>)> {
        self.prop_reads.lock().unwrap().clone()
    }

    fn prop_is_alive(&self, generation: i32) -> bool {
        self.props
            .lock()
            .unwrap()
            .iter()
            .filter(|(g, _)| *g == generation)
            .all(|(_, prop)| prop.try_get().is_some())
    }

    fn children<V>(
        &self,
        view: impl Fn(i32, RerunTest) -> V + Send + Sync + 'static,
    ) -> impl Fn() -> V + Send + Sync + 'static {
        let t = self.clone();
        move || view(t.generation.get(), t.clone())
    }
}

fn prop_created_in_closure(g: i32, t: RerunTest) -> impl Render {
    let prop = t.prop(g);
    let load = t.loads.load(g);
    Suspend::new(async move {
        load.await;
        t.view_reading_prop(g, prop)
    })
}

fn prop_created_in_future(g: i32, t: RerunTest) -> impl Render {
    let load = t.loads.load(g);
    Suspend::new(async move {
        load.await;
        let prop = t.prop(g);
        t.view_reading_prop(g, prop)
    })
}

fn prop_created_in_nested_suspend(g: i32, t: RerunTest) -> impl Render {
    let load = t.loads.load(g);
    Suspend::new(async move {
        load.await;
        Suspend::new(async move {
            let prop = t.prop(g);
            t.view_reading_prop(g, prop)
        })
    })
}

fn prop_created_in_child_owner(g: i32, t: RerunTest) -> impl Render {
    let load = t.loads.load(g);
    Suspend::new(async move {
        load.await;
        let owner = Owner::new();
        let prop = owner.with(|| t.prop(g));
        let mut view = t.view_reading_prop(g, prop);
        move || {
            let _owner = &owner;
            view()
        }
    })
}

fn assert_previous_view_stays_live<const TRANSITION: bool, V>(
    view: impl Fn(i32, RerunTest) -> V + Send + Sync + 'static,
) where
    V: Render + 'static,
    V::State: 'static,
{
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let t = RerunTest::new();
        let state =
            boundary::<TRANSITION, _>(t.children(view), Arc::clone(&t.drops))
                .build();
        poll_local_repeatedly();
        assert_eq!(t.prop_reads(), vec![(0, Some(0))]);

        t.loads.block(1);
        t.generation.set(1);
        poll_local_repeatedly();
        t.active.set(1);
        poll_local_repeatedly();
        assert_eq!(t.prop_reads(), vec![(0, Some(0)), (0, Some(0))]);
        assert!(t.prop_is_alive(0));

        t.loads.unblock(1);
        poll_local_repeatedly();
        assert_eq!(t.prop_reads().last(), Some(&(1, Some(1))));
        assert!(!t.prop_is_alive(0));

        let before = t.prop_reads().len();
        t.active.set(2);
        poll_local_repeatedly();
        assert_eq!(t.prop_reads()[before..], [(1, Some(1))]);

        drop(state);
        poll_local_repeatedly();
        assert!(!t.prop_is_alive(1));
    });
}

#[test]
fn suspense_keeps_previous_view_live_for_prop_created_in_closure() {
    assert_previous_view_stays_live::<false, _>(prop_created_in_closure);
}

#[test]
fn transition_keeps_previous_view_live_for_prop_created_in_closure() {
    assert_previous_view_stays_live::<true, _>(prop_created_in_closure);
}

#[test]
fn suspense_keeps_previous_view_live_for_prop_created_in_future() {
    assert_previous_view_stays_live::<false, _>(prop_created_in_future);
}

#[test]
fn transition_keeps_previous_view_live_for_prop_created_in_future() {
    assert_previous_view_stays_live::<true, _>(prop_created_in_future);
}

#[test]
fn suspense_keeps_previous_view_live_for_prop_created_in_nested_suspend() {
    assert_previous_view_stays_live::<false, _>(prop_created_in_nested_suspend);
}

#[test]
fn transition_keeps_previous_view_live_for_prop_created_in_nested_suspend() {
    assert_previous_view_stays_live::<true, _>(prop_created_in_nested_suspend);
}

#[test]
fn suspense_keeps_previous_view_live_for_prop_created_in_child_owner() {
    assert_previous_view_stays_live::<false, _>(prop_created_in_child_owner);
}

#[test]
fn transition_keeps_previous_view_live_for_prop_created_in_child_owner() {
    assert_previous_view_stays_live::<true, _>(prop_created_in_child_owner);
}

fn assert_superseded_rerun_is_disposed<const TRANSITION: bool>() {
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let t = RerunTest::new();
        let state = boundary::<TRANSITION, _>(
            t.children(prop_created_in_closure),
            Arc::clone(&t.drops),
        )
        .build();
        poll_local_repeatedly();

        // never unblocked
        t.loads.block(1);
        t.generation.set(1);
        poll_local_repeatedly();

        t.loads.block(2);
        t.generation.set(2);
        poll_local_repeatedly();
        assert!(!t.prop_is_alive(1));
        assert!(t.prop_is_alive(0));

        t.loads.unblock(2);
        poll_local_repeatedly();
        assert!(!t.prop_is_alive(0));
        assert!(t.prop_is_alive(2));
        let reads = t.prop_reads();
        assert!(
            reads.iter().all(|(g, value)| *g != 1 && value.is_some()),
            "{reads:?}"
        );

        drop(state);
        poll_local_repeatedly();
    });
}

#[test]
fn suspense_disposes_superseded_rerun() {
    assert_superseded_rerun_is_disposed::<false>();
}

#[test]
fn transition_disposes_superseded_rerun() {
    assert_superseded_rerun_is_disposed::<true>();
}

#[test]
fn reactive_function_cleans_up_before_rerun_and_disposes_after() {
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let generation = RwSignal::new(0);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let props = Arc::new(Mutex::new(Vec::<Signal<i32>>::new()));
        let drops = Arc::new(AtomicUsize::new(0));
        let state = {
            let log = Arc::clone(&log);
            let props = Arc::clone(&props);
            move || {
                let g = generation.get();
                let alive: Vec<_> = props
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|prop| prop.try_get().is_some())
                    .collect();
                log.lock()
                    .unwrap()
                    .push(format!("run {g}, alive {alive:?}"));
                props.lock().unwrap().push(Signal::stored(g));
                let log = Arc::clone(&log);
                on_cleanup(move || {
                    // the lock is poisoned if an assertion below has failed
                    if let Ok(mut log) = log.lock() {
                        log.push(format!("cleanup {g}"));
                    }
                });
                DropProbe(Arc::clone(&drops))
            }
        }
        .build();
        generation.set(1);
        poll_local_repeatedly();

        assert_eq!(
            *log.lock().unwrap(),
            ["run 0, alive []", "cleanup 0", "run 1, alive [true]"]
        );
        assert_eq!(props.lock().unwrap()[0].try_get(), None);

        drop(state);
        poll_local_repeatedly();
        assert_eq!(log.lock().unwrap().last().unwrap(), "cleanup 1");
    });
}

/// Pausing an ancestor owner pauses the render effects created during a run of a reactive
/// function.
#[test]
fn pausing_ancestor_pauses_effects_created_in_reactive_function() {
    _ = Executor::init_futures_executor();
    let root = Owner::new();
    root.with(|| {
        let active = RwSignal::new(0);
        let runs = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let state = {
            let runs = Arc::clone(&runs);
            move || {
                let runs = Arc::clone(&runs);
                let drops = Arc::clone(&drops);
                move || {
                    active.track();
                    runs.fetch_add(1, Ordering::SeqCst);
                    DropProbe(Arc::clone(&drops))
                }
            }
        }
        .build();
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        root.pause();
        active.set(1);
        poll_local_repeatedly();
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        root.resume();
        active.set(2);
        poll_local_repeatedly();
        assert_eq!(runs.load(Ordering::SeqCst), 2);

        drop(state);
    });
}
