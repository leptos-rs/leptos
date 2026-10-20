use super::*;
use any_spawner::Executor;
use reactive_graph::{signal::RwSignal, traits::Set};
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
