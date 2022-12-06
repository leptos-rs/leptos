use test::Bencher;

#[bench]
fn leptos_ssr_bench(b: &mut Bencher) {
	use leptos::*;

	b.iter(|| {
		_ = create_scope(create_runtime(), |cx| {
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
fn tera_ssr_bench(b: &mut Bencher) {
	use tera::*;
	use serde::{Serialize, Deserialize};

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

	lazy_static::lazy_static! { 
		static ref TERA: Tera = {
			let mut tera = Tera::default();
			tera.add_raw_templates(vec![("template.html", TEMPLATE)]).unwrap();
			tera
		};
	}

	#[derive(Serialize, Deserialize)]
	struct Counter {
		value: i32
	}

	b.iter(|| {
		let mut ctx = Context::new();
		ctx.insert("counters", &vec![
			Counter { value: 0 },
			Counter { value: 1},
			Counter { value: 2 }
		]);

		let _ = TERA.render("template.html", &ctx).unwrap();
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
