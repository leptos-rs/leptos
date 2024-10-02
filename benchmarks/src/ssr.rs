use test::Bencher;

#[bench]
fn leptos_ssr_bench(b: &mut Bencher) {
	use leptos::*;
	let r = create_runtime();
    b.iter(|| {
			leptos::leptos_dom::HydrationCtx::reset_id();
			#[component]
			fn Counter(initial: i32) -> impl IntoView {
				let (value, set_value) = create_signal(initial);
				view! {
					<div>
						<button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
						<span>"Value: " {move || value().to_string()} "!"</span>
						<button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
					</div>
				}
			}

			let rendered = view! {
				<main>
					<h1>"Welcome to our benchmark page."</h1>
					<p>"Here's some introductory text."</p>
					<Counter initial=1/>
					<Counter initial=2/>
					<Counter initial=3/>
				</main>
			}.into_view().render_to_string();

			assert_eq!(
				rendered,
"<main data-hk=\"0-0-0-1\"><h1 data-hk=\"0-0-0-2\">Welcome to our benchmark page.</h1><p data-hk=\"0-0-0-3\">Here&#x27;s some introductory text.</p><div data-hk=\"0-0-0-5\"><button data-hk=\"0-0-0-6\">-1</button><span data-hk=\"0-0-0-7\">Value: <!>1<!--hk=0-0-0-8-->!</span><button data-hk=\"0-0-0-9\">+1</button></div><!--hk=0-0-0-4--><div data-hk=\"0-0-0-11\"><button data-hk=\"0-0-0-12\">-1</button><span data-hk=\"0-0-0-13\">Value: <!>2<!--hk=0-0-0-14-->!</span><button data-hk=\"0-0-0-15\">+1</button></div><!--hk=0-0-0-10--><div data-hk=\"0-0-0-17\"><button data-hk=\"0-0-0-18\">-1</button><span data-hk=\"0-0-0-19\">Value: <!>3<!--hk=0-0-0-20-->!</span><button data-hk=\"0-0-0-21\">+1</button></div><!--hk=0-0-0-16--></main>"			);
	});
	r.dispose();
}

#[bench]
fn tachys_ssr_bench(b: &mut Bencher) {
	use leptos::{create_runtime, create_signal, SignalGet, SignalUpdate};
	use tachy_maccy::view;
	use tachydom::view::{Render, RenderHtml};
	use tachydom::html::element::ElementChild;
	use tachydom::html::attribute::global::ClassAttribute;
	use tachydom::html::attribute::global::GlobalAttributes;
	use tachydom::html::attribute::global::OnAttribute;
	use tachydom::renderer::dom::Dom;
	let rt = create_runtime();
    b.iter(|| {
		fn counter(initial: i32) -> impl Render<Dom> + RenderHtml<Dom> {
			let (value, set_value) = create_signal(initial);
			view! {
				<div>
					<button on:click=move |_| set_value.update(|value| *value -= 1)>"-1"</button>
					<span>"Value: " {move || value().to_string()} "!"</span>
					<button on:click=move |_| set_value.update(|value| *value += 1)>"+1"</button>
				</div>
			}
		}

		let rendered = view! {
			<main>
				<h1>"Welcome to our benchmark page."</h1>
				<p>"Here's some introductory text."</p>
				{counter(1)}
				{counter(2)}
				{counter(3)}
			</main>
		}.to_html();
		assert_eq!(
			rendered,
			"<main><h1>Welcome to our benchmark page.</h1><p>Here's some introductory text.</p><div><button>-1</button><span>Value: <!>1<!>!</span><button>+1</button></div><div><button>-1</button><span>Value: <!>2<!>!</span><button>+1</button></div><div><button>-1</button><span>Value: <!>3<!>!</span><button>+1</button></div></main>"
		);
	});
	rt.dispose();
}

#[bench]
fn tera_ssr_bench(b: &mut Bencher) {
    use serde::{Deserialize, Serialize};
    use tera::*;

    static TEMPLATE: &str = r#"<main>
	<h1>Welcome to our benchmark page.</h1>
	<p>Here's some introductory text.</p>
	{% for counter in counters %}
	<div>
		<button>-1</button>
		<span>Value: {{ counter.value }}!</span>
		<button>+1</button>
	</div>
	{% endfor %}
	</main>"#;


    static  LazyCell<TERA>: Tera = LazyLock::new(|| {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![("template.html", TEMPLATE)]).unwrap();
        tera
    });


    #[derive(Serialize, Deserialize)]
    struct Counter {
        value: i32,
    }

    b.iter(|| {
        let mut ctx = Context::new();
        ctx.insert(
            "counters",
            &vec![
                Counter { value: 0 },
                Counter { value: 1 },
                Counter { value: 2 },
            ],
        );

        let _ = TERA.render("template.html", &ctx).unwrap();
    });
}

#[bench]
fn sycamore_ssr_bench(b: &mut Bencher) {
    use sycamore::prelude::*;
    use sycamore::*;

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
