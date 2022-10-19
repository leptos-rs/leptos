#![feature(test)]

extern crate test;
/* 
mod reactive {
    use test::Bencher;

    use std::{cell::Cell, rc::Rc};

    #[bench]
    fn leptos_create_1000_signals(b: &mut Bencher) {
        use leptos::{create_isomorphic_effect, create_memo, create_scope, create_signal};

        b.iter(|| {
            create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
                let writes = sigs.iter().map(|(_, w)| *w).collect::<Vec<_>>();
                let memo = create_memo(cx, move |_| reads.iter().map(|r| r.get()).sum::<i32>());
                assert_eq!(memo(), 499500);
            })
            .dispose()
        });
    }

    #[bench]
    fn leptos_create_and_update_1000_signals(b: &mut Bencher) {
        use leptos::{create_isomorphic_effect, create_memo, create_scope, create_signal};

        b.iter(|| {
            create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
                let writes = sigs.iter().map(|(_, w)| *w).collect::<Vec<_>>();
                let memo = create_memo(cx, move |_| reads.iter().map(|r| r.get()).sum::<i32>());
                assert_eq!(memo(), 499500);
                create_isomorphic_effect(cx, {
                    let acc = Rc::clone(&acc);
                    move |_| {
                        acc.set(memo());
                    }
                });
                assert_eq!(acc.get(), 499500);

                writes[1].update(|n| *n += 1);
                writes[10].update(|n| *n += 1);
                writes[100].update(|n| *n += 1);

                assert_eq!(acc.get(), 499503);
                assert_eq!(memo(), 499503);
            })
            .dispose()
        });
    }

    #[bench]
    fn leptos_create_and_dispose_1000_scopes(b: &mut Bencher) {
        use leptos::{create_isomorphic_effect, create_scope, create_signal};

        b.iter(|| {
            let acc = Rc::new(Cell::new(0));
            let disposers = (0..1000)
                .map(|_| {
                    create_scope({
                        let acc = Rc::clone(&acc);
                        move |cx| {
                            let (r, w) = create_signal(cx, 0);
                            create_isomorphic_effect(cx, {
                                move |_| {
                                    acc.set(r());
                                }
                            });
                            w.update(|n| *n += 1);
                        }
                    })
                })
                .collect::<Vec<_>>();
            for disposer in disposers {
                disposer.dispose();
            }
        });
    }

    #[bench]
    fn sycamore_create_1000_signals(b: &mut Bencher) {
        use sycamore::reactive::{create_effect, create_memo, create_scope, create_signal};

        b.iter(|| {
            let d = create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = Rc::new((0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>());
                let memo = create_memo(cx, {
                    let sigs = Rc::clone(&sigs);
                    move || sigs.iter().map(|r| *r.get()).sum::<i32>()
                });
                assert_eq!(*memo.get(), 499500);
            });
            unsafe { d.dispose() };
        });
    }

    #[bench]
    fn sycamore_create_and_update_1000_signals(b: &mut Bencher) {
        use sycamore::reactive::{create_effect, create_memo, create_scope, create_signal};

        b.iter(|| {
            let d = create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = Rc::new((0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>());
                let memo = create_memo(cx, {
                    let sigs = Rc::clone(&sigs);
                    move || sigs.iter().map(|r| *r.get()).sum::<i32>()
                });
                assert_eq!(*memo.get(), 499500);
                create_effect(cx, {
                    let acc = Rc::clone(&acc);
                    move || {
                        acc.set(*memo.get());
                    }
                });
                assert_eq!(acc.get(), 499500);

                sigs[1].set(*sigs[1].get() + 1);
                sigs[10].set(*sigs[10].get() + 1);
                sigs[100].set(*sigs[100].get() + 1);

                assert_eq!(acc.get(), 499503);
                assert_eq!(*memo.get(), 499503);
            });
            unsafe { d.dispose() };
        });
    }

    #[bench]
    fn sycamore_create_and_dispose_1000_scopes(b: &mut Bencher) {
        use sycamore::reactive::{create_effect, create_scope, create_signal};

        b.iter(|| {
            let acc = Rc::new(Cell::new(0));
            let disposers = (0..1000)
                .map(|_| {
                    create_scope({
                        let acc = Rc::clone(&acc);
                        move |cx| {
                            let s = create_signal(cx, 0);
                            create_effect(cx, {
                                move || {
                                    acc.set(*s.get());
                                }
                            });
                            s.set(*s.get() + 1);
                        }
                    })
                })
                .collect::<Vec<_>>();
            for disposer in disposers {
                unsafe {
                    disposer.dispose();
                }
            }
        });
    }
} */

mod ssr {
    use test::Bencher;

    #[bench]
    fn leptos_ssr_bench(b: &mut Bencher) {
        use leptos::*;

        b.iter(|| {
            _ = create_scope(|cx| {
                #[component]
				fn Counter(cx: Scope, initial: i32) -> Element {
					let (value, set_value) = create_signal(cx, initial);
					view! {
						cx,
						<div>
							<button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
							<span>"Value: " {move || value().to_string()} "!"</span>
							<button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
						</div>
					}
				}

				let rendered = view! { 
					cx,
					<main>
                        <h1>"Welcome to our benchmark page."</h1>
                        <p>"Here's some introductory text."</p>
						<Counter initial=1/>
						<Counter initial=2/>
						<Counter initial=3/>
					</main>
				};

                assert_eq!(
                    rendered,
                    "<main data-hk=\"0-0\"><h1>Welcome to our benchmark page.</h1><p>Here's some introductory text.</p><!--#--><div data-hk=\"0-2-0\"><button>-1</button><span>Value: <!--#-->1<!--/-->!</span><button>+1</button></div><!--/--><!--#--><div data-hk=\"0-3-0\"><button>-1</button><span>Value: <!--#-->2<!--/-->!</span><button>+1</button></div><!--/--><!--#--><div data-hk=\"0-4-0\"><button>-1</button><span>Value: <!--#-->3<!--/-->!</span><button>+1</button></div><!--/--></main>"
                );
            });
        });
    }

    #[bench]
    fn sycamore_ssr_bench(b: &mut Bencher) {
        use sycamore::*;
        use sycamore::prelude::*;

        b.iter(|| {
            _ = create_scope(|cx| {
                #[derive(Prop)]
                struct CounterProps {
                    initial: i32
                }


                #[component]
				fn Counter<G: Html>(cx: Scope, props: CounterProps) -> View<G> {
					let value = create_signal(cx, props.initial);
					view! {
						cx,
                        div {
                            button(on:click=|_| value.set(*value.get() - 1)) {
                                "-1"
                            }
                            span {
                                "Value: "
                                (value.get().to_string())
                                "!"
                            }
                            button(on:click=|_| value.set(*value.get() + 1)) {
                                "+1"
                            }
                        }
					}
				}

				let rendered = render_to_string(|cx| view! {
                    cx,
                    main {
                        h1 {
                            "Welcome to our benchmark page."
                        }
                        p {
                            "Here's some introductory text."
                        }
                        Counter(initial = 1)
                        Counter(initial = 2)
                        Counter(initial = 3)
                    }
                });

                assert_eq!(
                    rendered,
                    "<main data-hk=\"0.0\"><h1 data-hk=\"0.1\">Welcome to our benchmark page.</h1><p data-hk=\"0.2\">Here's some introductory text.</p><!--#--><div data-hk=\"1.0\"><button data-hk=\"1.1\">-1</button><span data-hk=\"1.2\">Value: <!--#-->1<!--/-->!</span><button data-hk=\"1.3\">+1</button></div><!--/--><!----><!--#--><div data-hk=\"2.0\"><button data-hk=\"2.1\">-1</button><span data-hk=\"2.2\">Value: <!--#-->2<!--/-->!</span><button data-hk=\"2.3\">+1</button></div><!--/--><!----><!--#--><div data-hk=\"3.0\"><button data-hk=\"3.1\">-1</button><span data-hk=\"3.2\">Value: <!--#-->3<!--/-->!</span><button data-hk=\"3.3\">+1</button></div><!--/--></main>"
                );
            });
        });
    }

    #[bench]
    fn yew_ssr_bench(b: &mut Bencher) {
        use yew::prelude::*;
        use yew::ServerRenderer;

        b.iter(|| {
            #[derive(Properties, PartialEq, Eq, Debug)]
            struct CounterProps {
                initial: i32
            }

            #[function_component(Counter)]
            fn counter(props: &CounterProps) -> Html {
                let state = use_state(|| props.initial);

                let incr_counter = {
                    let state = state.clone();
                    Callback::from(move |_| state.set(&*state + 1))
                };

                let decr_counter = {
                    let state = state.clone();
                    Callback::from(move |_| state.set(&*state - 1))
                };

                html! {
                    <div>
                        <h1>{"Welcome to our benchmark page."}</h1>
                        <p>{"Here's some introductory text."}</p>
                        <button onclick={decr_counter}> {"-1"} </button>
                        <p> {"Value: "} {*state} {"!"} </p>
                        <button onclick={incr_counter}> {"+1"} </button>
                    </div>
                }
            }

            #[function_component]
            fn App() -> Html {
                html! {
                    <main>
                        <Counter initial=1/>
                        <Counter initial=2/>
                        <Counter initial=3/>
                    </main>
                }
            }

            tokio_test::block_on(async {
                let renderer = ServerRenderer::<App>::new();
                let rendered = renderer.render().await;
                assert_eq!(
                    rendered,
                    "<!--<[]>--><main><!--<[]>--><div><h1>Welcome to our benchmark page.</h1><p>Here's some introductory text.</p><button>-1</button><p>Value: 1!</p><button>+1</button></div><!--</[]>--><!--<[]>--><div><h1>Welcome to our benchmark page.</h1><p>Here's some introductory text.</p><button>-1</button><p>Value: 2!</p><button>+1</button></div><!--</[]>--><!--<[]>--><div><h1>Welcome to our benchmark page.</h1><p>Here's some introductory text.</p><button>-1</button><p>Value: 3!</p><button>+1</button></div><!--</[]>--></main><!--</[]>-->"
                );
            });
        });
    }
}