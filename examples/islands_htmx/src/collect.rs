//! Frees an island once it has left the page, which Leptos's `#[island]` macro never does.

use leptos::{
    prelude::*,
    tachys::{
        html::attribute::{any_attribute::AnyAttribute, Attribute},
        hydration::Cursor,
        renderer::types::{Element, Node},
        ssr::StreamBuilder,
        view::{
            add_attr::AddAnyAttr,
            any_view::{AnyView, AnyViewState, IntoAny},
            Mountable, Position, PositionState, Render, RenderHtml,
        },
    },
};

/// Wraps an island's body so the sweep can free the island later; every island's body goes through here.
pub fn collected<V: IntoView>(body: impl FnOnce() -> V) -> Collected {
    let owner = Owner::current().unwrap_or_default();
    let view = owner.with(|| body().into_any());
    Collected { view, owner }
}

/// The island's view, type-erased so a debug build cannot unwrap it, and the owner the sweep cleans up.
pub struct Collected {
    view: AnyView,
    owner: Owner,
}

/// What Leptos gets to keep: the real state when the view is built in the browser, nothing once hydrated.
pub struct CollectedState(Option<AnyViewState>);

impl Render for Collected {
    type State = CollectedState;

    fn build(self) -> Self::State {
        CollectedState(Some(self.view.build()))
    }

    fn rebuild(self, state: &mut Self::State) {
        if let Some(state) = &mut state.0 {
            self.view.rebuild(state);
        }
    }
}

impl AddAnyAttr for Collected {
    type Output<SomeNewAttr: Attribute> = Collected;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        Collected {
            view: self.view.add_any_attr(attr).into_any(),
            owner: self.owner,
        }
    }
}

impl RenderHtml for Collected {
    type AsyncOutput = Collected;
    type Owned = Collected;

    const MIN_LENGTH: usize = <AnyView as RenderHtml>::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        Collected {
            view: self.view.resolve().await,
            owner: self.owner,
        }
    }

    fn html_len(&self) -> usize {
        self.view.html_len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        self.view.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
    }

    /// Hydrates the view, keeps its state and the owner in the registry, and hands Leptos an empty state to forget.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let island = cursor.current();
        let state = self.view.hydrate::<FROM_SERVER>(cursor, position);
        #[cfg(feature = "hydrate")]
        {
            sweep::register(island, self.owner, state);
            CollectedState(None)
        }
        #[cfg(not(feature = "hydrate"))]
        {
            let _ = (island, self.owner);
            CollectedState(Some(state))
        }
    }

    /// Returns itself: a debug build calls this before hydrating, and a wrapper unwrapped here would never register.
    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl Mountable for CollectedState {
    fn unmount(&mut self) {
        if let Some(state) = &mut self.0 {
            state.unmount();
        }
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        if let Some(state) = &mut self.0 {
            state.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.0
            .as_ref()
            .is_some_and(|state| state.insert_before_this(child))
    }

    fn elements(&self) -> Vec<Element> {
        self.0.as_ref().map(Mountable::elements).unwrap_or_default()
    }
}

/// The registry of hydrated islands and the sweep that frees the ones gone from the page; browser only.
#[cfg(feature = "hydrate")]
mod sweep {
    use leptos::{
        prelude::*, tachys::view::any_view::AnyViewState, web_sys::Node,
    };
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    /// How often the sweep runs: rarely in release, often in a debug build so a check need not wait.
    const EVERY: Duration = if cfg!(debug_assertions) {
        Duration::from_secs(2)
    } else {
        Duration::from_secs(30)
    };

    /// Sweeps in a row an island must be off the page before it is freed; two, so a brief move does not lose it.
    const STRIKES: u8 = 2;

    struct Entry {
        island: Node,
        owner: Owner,
        state: AnyViewState,
        missing: u8,
    }

    thread_local! {
        static LIVE: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
        static STARTED: Cell<bool> = const { Cell::new(false) };
    }

    /// Keeps a hydrated island; the first one starts the sweep, which runs for the page's life.
    pub(super) fn register(island: Node, owner: Owner, state: AnyViewState) {
        LIVE.with_borrow_mut(|live| {
            live.push(Entry {
                island,
                owner,
                state,
                missing: 0,
            })
        });
        // The tests sweep by hand, so no timer runs there.
        if !cfg!(test) && !STARTED.replace(true) {
            set_interval(sweep, EVERY);
        }
    }

    /// Counts a strike for each island off the page, clears it for each one back on it, and frees those at the limit.
    fn sweep() {
        let gone: Vec<Entry> = LIVE.with_borrow_mut(|live| {
            for entry in live.iter_mut() {
                if entry.island.is_connected() {
                    entry.missing = 0;
                } else {
                    entry.missing += 1;
                }
            }
            let (gone, kept) = std::mem::take(live)
                .into_iter()
                .partition(|entry| entry.missing >= STRIKES);
            *live = kept;
            gone
        });
        let freed = gone.len();
        // Outside the borrow, since an island's cleanup may touch the registry; the view first, then the owner.
        for Entry { owner, state, .. } in gone {
            drop(state);
            owner.cleanup();
        }
        if cfg!(debug_assertions) && freed > 0 {
            let live = LIVE.with_borrow(Vec::len);
            leptos::logging::log!("islands: {live} live, {freed} freed");
        }
    }

    /// In headless Chrome. Each test builds an island as the macro does and sweeps by hand; they share one registry.
    #[cfg(all(test, target_arch = "wasm32"))]
    mod tests {
        use super::{sweep, LIVE};
        use crate::collect::collected;
        use leptos::{
            prelude::*,
            tachys::{
                reactive_graph::OwnedView,
                view::{any_view::IntoAny, Position, RenderHtml},
            },
            wasm_bindgen::JsCast,
            web_sys::HtmlElement,
        };
        use std::{
            cell::Cell,
            rc::Rc,
            sync::{
                atomic::{AtomicBool, Ordering},
                Arc,
            },
        };
        use wasm_bindgen_test::{
            wasm_bindgen_test, wasm_bindgen_test_configure,
        };

        wasm_bindgen_test_configure!(run_in_browser);

        struct Island {
            element: HtmlElement,
            signal: RwSignal<u8>,
            cleaned: Arc<AtomicBool>,
        }

        /// Hydrates an island in a new div holding the empty view's marker; `erased` goes through `into_any()` first, as a debug build does.
        fn hydrated(erased: bool) -> Island {
            let element: HtmlElement =
                document().create_element("div").unwrap().unchecked_into();
            element.set_inner_html(&().to_html());
            document().body().unwrap().append_child(&element).unwrap();
            let cleaned = Arc::new(AtomicBool::new(false));
            let signal = Rc::new(Cell::new(None));
            let owner = Owner::new();
            let view = owner.with(|| {
                let (cleaned, signal) =
                    (Arc::clone(&cleaned), Rc::clone(&signal));
                OwnedView::new(collected(move || {
                    signal.set(Some(RwSignal::new(1_u8)));
                    on_cleanup(move || cleaned.store(true, Ordering::SeqCst));
                }))
            });
            if erased {
                std::mem::forget(
                    view.into_any().hydrate_from_position::<true>(
                        &element,
                        Position::FirstChild,
                    ),
                );
            } else {
                std::mem::forget(view.hydrate_from_position::<true>(
                    &element,
                    Position::FirstChild,
                ));
            }
            Island {
                element,
                signal: signal.get().unwrap(),
                cleaned,
            }
        }

        fn live() -> usize {
            LIVE.with_borrow(Vec::len)
        }

        fn drain() {
            let entries = LIVE.with_borrow_mut(std::mem::take);
            drop(entries);
        }

        fn cleaned(island: &Island) -> bool {
            island.cleaned.load(Ordering::SeqCst)
        }

        #[wasm_bindgen_test]
        fn freed_after_two_sweeps() {
            drain();
            let island = hydrated(false);
            assert_eq!(live(), 1, "registered when hydrated");
            island.element.remove();
            sweep();
            assert_eq!(live(), 1, "one sweep off the page is one strike");
            sweep();
            assert_eq!(live(), 0, "freed at the second strike");
            assert!(cleaned(&island), "its on_cleanup ran");
            assert_eq!(
                island.signal.try_get_untracked(),
                None,
                "its signal is disposed"
            );
            drain();
        }

        #[wasm_bindgen_test]
        fn kept_while_on_the_page() {
            drain();
            let island = hydrated(false);
            sweep();
            sweep();
            assert_eq!(live(), 1);
            assert!(!cleaned(&island));
            assert_eq!(island.signal.try_get_untracked(), Some(1));
            island.element.remove();
            drain();
        }

        #[wasm_bindgen_test]
        fn one_strike_then_back_is_kept() {
            drain();
            let island = hydrated(false);
            island.element.remove();
            sweep();
            document()
                .body()
                .unwrap()
                .append_child(&island.element)
                .unwrap();
            sweep();
            sweep();
            assert_eq!(live(), 1, "back on the page clears the strike");
            assert!(!cleaned(&island));
            island.element.remove();
            drain();
        }

        #[wasm_bindgen_test]
        fn survives_type_erasure() {
            drain();
            let island = hydrated(true);
            assert_eq!(live(), 1, "registered through into_any()");
            island.element.remove();
            sweep();
            sweep();
            assert_eq!(live(), 0);
            assert!(cleaned(&island));
            drain();
        }
    }
}
