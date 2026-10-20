use super::*;
use any_spawner::Executor;
use std::sync::atomic::{AtomicUsize, Ordering};
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

fn assert_synchronous_disposal(with_error: bool) {
    _ = Executor::init_futures_executor();
    Owner::new().with(|| {
        let drops = Arc::new(AtomicUsize::new(0));
        let hook = Arc::new(ErrorBoundaryErrorHook::new(
            Default::default(),
            Vec::new(),
        ));
        if with_error {
            hook.throw(std::io::Error::other("test error").into());
        }
        let errors = hook.errors.clone();
        let errors_empty = ArcMemo::new({
            let errors = errors.clone();
            move |_| errors.with(|map| map.is_empty())
        });
        let state = ErrorBoundaryView {
            hook,
            boundary_id: Default::default(),
            errors_empty,
            children: DropProbe(Arc::clone(&drops)),
            fallback: {
                let drops = Arc::clone(&drops);
                move |_| DropProbe(Arc::clone(&drops))
            },
            errors,
            suspended_children: Default::default(),
        }
        .build();
        Executor::poll_local();

        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(state);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            if with_error { 2 } else { 1 },
            "children and fallback must be dropped before the executor runs"
        );
        Executor::poll_local();
    });
}

#[test]
fn drops_children_synchronously() {
    assert_synchronous_disposal(false);
}

#[test]
fn drops_children_and_active_fallback_synchronously() {
    assert_synchronous_disposal(true);
}
