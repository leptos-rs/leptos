use super::*;
use any_spawner::Executor;
use reactive_graph::{signal::RwSignal, traits::Set, wrappers::read::Signal};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tachys::renderer::types::{Element, Node};

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

#[derive(Clone)]
struct OwnerTest {
    outer: RwSignal<i32>,
    reads: Arc<Mutex<Vec<Option<i32>>>>,
    drops: Arc<AtomicUsize>,
}

impl OwnerTest {
    fn view(
        self,
        prop: Signal<i32>,
    ) -> impl FnMut() -> DropProbe + Send + 'static {
        move || {
            self.outer.track();
            self.reads.lock().unwrap().push(prop.try_get());
            DropProbe(Arc::clone(&self.drops))
        }
    }
}

/// When the boundary's children rerun and the new `Suspend` is pending, the
/// previous view stays mounted, but the owner it was created in has been
/// cleaned up. Its bindings must not rerun, or they read disposed values.
fn assert_pending_rerun_pauses_previous_view<const TRANSITION: bool, V>(
    view: impl Fn(i32, OwnerTest) -> V + Send + Sync + 'static,
) where
    V: Render + 'static,
    V::State: 'static,
{
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let generation = RwSignal::new(0);
        let cx = OwnerTest {
            outer: RwSignal::new(0),
            reads: Arc::new(Mutex::new(Vec::new())),
            drops: Arc::new(AtomicUsize::new(0)),
        };
        let state = boundary::<TRANSITION, _>(
            {
                let cx = cx.clone();
                move || view(generation.get(), cx.clone())
            },
            Arc::clone(&cx.drops),
        )
        .build();
        Executor::poll_local();
        assert_eq!(*cx.reads.lock().unwrap(), vec![Some(0)]);

        generation.set(1);
        Executor::poll_local();
        cx.outer.set(1);
        Executor::poll_local();

        assert_eq!(
            *cx.reads.lock().unwrap(),
            vec![Some(0)],
            "the previous view's binding read a disposed value"
        );
        drop(state);
        Executor::poll_local();
    });
}

async fn load(generation: i32) {
    if generation > 0 {
        futures::future::pending::<()>().await;
    }
}

fn previous_view<const TRANSITION: bool>() {
    use reactive_graph::wrappers::read::Signal;
    use tachys::reactive_graph::Suspend;

    assert_pending_rerun_pauses_previous_view::<TRANSITION, _>(|g, probe| {
        let prop = Signal::stored(g);
        Suspend::new(async move {
            load(g).await;
            probe.view(prop)
        })
    });
}

#[test]
fn suspense_pauses_previous_view_while_rerun_is_pending() {
    previous_view::<false>();
}

#[test]
fn transition_pauses_previous_view_while_rerun_is_pending() {
    previous_view::<true>();
}

fn previous_owned_view<const TRANSITION: bool>() {
    use reactive_graph::wrappers::read::Signal;
    use tachys::reactive_graph::{OwnedView, Suspend};

    assert_pending_rerun_pauses_previous_view::<TRANSITION, _>(|g, probe| {
        Suspend::new(async move {
            load(g).await;
            let owner = Owner::current().expect("no reactive owner").child();
            let prop = owner.with(|| Signal::stored(g));
            OwnedView::new_with_owner(probe.view(prop), owner)
        })
    });
}

#[test]
fn suspense_pauses_previous_owned_view_while_rerun_is_pending() {
    previous_owned_view::<false>();
}

#[test]
fn transition_pauses_previous_owned_view_while_rerun_is_pending() {
    previous_owned_view::<true>();
}

fn previous_nested_suspend<const TRANSITION: bool>() {
    use reactive_graph::wrappers::read::Signal;
    use tachys::reactive_graph::Suspend;

    assert_pending_rerun_pauses_previous_view::<TRANSITION, _>(|g, probe| {
        Suspend::new(async move {
            load(g).await;
            Suspend::new(async move { probe.view(Signal::stored(g)) })
        })
    });
}

#[test]
fn suspense_pauses_previous_nested_suspend_while_rerun_is_pending() {
    previous_nested_suspend::<false>();
}

#[test]
fn transition_pauses_previous_nested_suspend_while_rerun_is_pending() {
    previous_nested_suspend::<true>();
}
