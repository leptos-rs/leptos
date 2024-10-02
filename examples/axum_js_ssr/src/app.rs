use crate::{
    api::fetch_code,
    consts::{CH03_05A, LEPTOS_HYDRATED},
};
use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{FlatRoutes, Route, Router, A},
    path, SsrMode,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();

    view! {
        <Stylesheet id="leptos" href="/pkg/axum_js_ssr.css"/>
        <Title text="Leptos JavaScript Integration Demo with SSR in Axum"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <nav>
                <A attr:class="section" href="/">"Introduction (home)"</A>
                <A attr:class="example" href="/naive">"Naive "<code>"<script>"</code>
                    <small>"truly naive to start off"</small></A>
                <A attr:class="example" href="/naive-alt">"Leptos "<code>"<Script>"</code>
                    <small>"naively using load event"</small></A>
                <A attr:class="example" href="/naive-hook">"Leptos "<code>"<Script>"</code>
                    <small>"... correcting placement"</small></A>
                <A attr:class="example" href="/naive-fallback">"Leptos "<code>"<Script>"</code>
                    <small>"... with fallback"</small></A>
                <A attr:class="example" href="/signal-effect-script">"Leptos Signal + Effect"
                    <small>"an idiomatic Leptos solution"</small></A>
                <A attr:class="subexample section" href="/custom-event">"Hydrated Event"
                    <small>"using "<code>"js_sys"</code>"/"<code>"web_sys"</code></small></A>
                <A attr:class="example" href="/wasm-bindgen-naive">"Using "<code>"wasm-bindgen"</code>
                    <small>"naively to start with"</small></A>
                <A attr:class="example" href="/wasm-bindgen-event">"Using "<code>"wasm-bindgen"</code>
                    <small>"overcomplication with events"</small></A>
                <A attr:class="example" href="/wasm-bindgen-effect">"Using "<code>"wasm-bindgen"</code>
                    <small>"lazily delay DOM manipulation"</small></A>
                <A attr:class="example" href="/wasm-bindgen-direct">"Using "<code>"wasm-bindgen"</code>
                    <small>"without DOM manipulation"</small></A>
                <A attr:class="example section" href="/wasm-bindgen-direct-fixed">
                    "Using "<code>"wasm-bindgen"</code>
                    <small>"corrected with signal + effect"</small>
                </A>
                <a id="reset" href="/" target="_self">"Restart/Rehydrate"
                    <small>"to make things work again"</small></a>
            </nav>
            <main>
                <div id="notice">
                    "The WASM application has panicked during hydration. "
                    <a href="/" target="_self">
                        "Restart the application by going home"
                    </a>"."
                </div>
                <article>
                    <h1>"Leptos JavaScript Integration Demo with SSR in Axum"</h1>
                    <FlatRoutes fallback>
                        <Route path=path!("") view=HomePage/>
                        <Route path=path!("naive") view=Naive ssr=SsrMode::Async/>
                        <Route path=path!("naive-alt") view=|| view! { <NaiveEvent/> } ssr=SsrMode::Async/>
                        <Route path=path!("naive-hook") view=|| view! { <NaiveEvent hook=true/> } ssr=SsrMode::Async/>
                        <Route path=path!("naive-fallback") view=|| view! {
                            <NaiveEvent hook=true fallback=true/>
                        } ssr=SsrMode::Async/>
                        <Route path=path!("signal-effect-script") view=CodeDemoSignalEffect ssr=SsrMode::Async/>
                        <Route path=path!("custom-event") view=CustomEvent ssr=SsrMode::Async/>
                        <Route path=path!("wasm-bindgen-naive") view=WasmBindgenNaive ssr=SsrMode::Async/>
                        <Route path=path!("wasm-bindgen-event") view=WasmBindgenJSHookReadyEvent ssr=SsrMode::Async/>
                        <Route path=path!("wasm-bindgen-effect") view=WasmBindgenEffect ssr=SsrMode::Async/>
                        <Route path=path!("wasm-bindgen-direct") view=WasmBindgenDirect ssr=SsrMode::Async/>
                        <Route path=path!("wasm-bindgen-direct-fixed") view=WasmBindgenDirectFixed ssr=SsrMode::Async/>
                    </FlatRoutes>
                </article>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <p>"
            This example application demonstrates a number of ways that JavaScript may be included and used
            with Leptos naively, describing and showing the shortcomings and failures associated with each of
            them for both SSR (Server-Side Rendering) and CSR (Client-Side Rendering) with hydration, before
            leading up to the idiomatic solutions where they work as expected.
        "</p>
        <p>"
            For the demonstrations, "<a href="https://github.com/highlightjs/highlight.js"><code>
            "highlight.js"</code></a>" will be invoked from within this Leptos application by the examples
            linked on the side bar.  Since the library to be integrated is a JavaScript library, it must be
            enabled to fully appreciate this demo, and having the browser's developer tools/console opened is
            recommended as the logs will indicate the effects and issues as they happen.
        "</p>
        <p>"
            Examples 1 to 5 are primarily JavaScript based, where the integration code is included as "<code>
            "<script>"</code>" tags, with example 5 (final example of the group) being the idiomatic solution
            that runs without errors or panic during hydration, plus an additional example 5.1 showing how to
            get hydration to dispatch an event for JavaScript libraries should that be required.  Examples 6
            to 10 uses "<code>"wasm-bindgen"</code>" to call out to the JavaScript library from Rust, starting
            off with naive examples that mimics JavaScript conventions, again with the final example of the
            group (example 10) being the fully working version that embraces the use of Rust.
        "</p>
    }
}

#[derive(Clone, Debug)]
struct CodeDemoHook {
    js_hook: String,
}

#[component]
fn CodeDemo() -> impl IntoView {
    let code = Resource::new(|| (), |_| fetch_code());
    let code_view = move || {
        Suspend::new(async move {
            let hook = use_context::<CodeDemoHook>().map(|h| {
                leptos::logging::log!("use context suspend JS");
                view! {
                    <Script>{h.js_hook}</Script>
                }
            });
            view! {
                <pre><code class="language-rust">{code.await}</code></pre>
                {hook}
            }
        })
    };
    view! {
        <p>"Explanation on what is being demonstrated follows after the following code example table."</p>
        <div id="code-demo">
            <table>
                <thead>
                    <tr>
                        <th>"Inline code block (part of this component)"</th>
                        <th>"Dynamic code block (loaded via server fn)"</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td><pre><code class="language-rust">{CH03_05A}</code></pre></td>
                        <td>
                            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>
                                {code_view}
                            </Suspense>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn Naive() -> impl IntoView {
    let loader = r#"<script src="/highlight.min.js"></script>
<script>hljs.highlightAll();</script>"#;
    view! {
        <h2>"Showing what happens when script inclusion is done naively"</h2>
        <CodeDemo/>
        <p>"
            This page demonstrates what happens (or doesn't happen) when it is assumed that the "<code>
            "highlight.js"</code>" library can just be included from some CDN (well, hosted locally for this
            example) as per their instructions for basic usage in the browser, specifically:
        "</p>
        <div><pre><code class="language-html">{loader}</code></pre></div>
        <p>"
            The following actions should be taken in order to fully experience the things that do not work as
            expected:
        "</p>
        <ol>
            <li>"
                You may find that during the initial load of this page when first navigating to here from
                \"Introduction\" (do navigate there, reload to reinitiate this application to properly
                replicate the behavior, or simply use the Restart link at the bottom), none of the code
                examples below are highlighted.
            "</li>
            <li>"
                Go back and then forward again using the browser's navigation system the inline code block
                will become highlighted.  The cause is due to "<code>"highlight.js"</code>" being loaded in a
                standard "<code>"<script>"</code>" tag that is part of this component and initially it wasn't
                loaded before the call to "<code>"hljs.highlightAll();"</code>" was made. Later, when the
                component gets re-rendered the second time, the code is finally available to ensure one of
                them works (while also reloading the script, which probably isn't desirable for this use
                case).
            "</li>
            <li>"
                If you have the browser reload this page, you will find that "<strong>"both"</strong>" code
                examples now appear to highlight correctly, yay! However you will also find that the browser's
                back button appears to do nothing at all (even though the address bar may have changed), and
                that most of the links on the side-bar are non-functional.  A message should have popped up at
                the top indicating that the application has panicked.
                "<details>"
                    "<summary>"Details about the cause of the crash:"</summary>
                    <p>"
                        The cause here is because the hydration system found a node where text was expected, a
                        simple violation of the application's invariant.  Specifically, the code block
                        originally contained plain text, but with highlighting that got changed to some HTML
                        markup "<em>"before"</em>" hydration happened, completely ouside of expectations.
                        Generally speaking, a panic is the worst kind of error, as it is a hard crash which
                        stops the application from working, and in this case the reactive system is in a
                        completely non-functional state.
                    "</p>
                    <p>"
                        Fortunately for this application, some internal links within this application have
                        been specifically excluded from the reactive system (specifically the restart links,
                        so they remain usable as they are just standard links which include the bottommost one
                        of the side bar and the one that should become visible as a notification as the panic
                        happened at the top - both may be used to navigate non-reactively back to the
                        homepage.
                    "</p>
                    <p>"
                        Navigating back after using the non-reactive links will also restart the application,
                        so using that immediately after to return to this page will once again trigger the
                        same condition that will result the hydration to panic.  If you wish to maintain the
                        push state within the history, simply use the browser navigation to navigate through
                        those pushed addresses and find one that may be reloaded without causing the crash,
                        and then go the opposite direction the same number of steps to get back to here.
                    "</p>"
                "</details>"
            "</li>
            <li>"
                In the working CSR state, if you continue to use the browser's navigation system to go back to
                home and forward back to this page, you will find that the the browser's console log is
                spammed with the different delays added to the loading of the standard highlight.js file.  The
                cause is because the script is unloaded/reloaded every time its "<code>"<script>"</code>" tag
                is re-created by this component.  This may or may not be a desirable behavior, so where
                exactly these tags are situated will matter - if the goal is to load the script once, the tag
                should be provided above the Router.
            "</li>
            <li>"
                Simply use the restart links to get back home and move onto the next example - or come back
                here, if you wish - while all the examples can be used out of order, the intended broken
                behaviors being demonstrated are best experienced by going home using the reactive link at the
                top, and go back to the target example.  Going between different examples demonstrating the
                subtly broken behavior(s) in arbitrary order can and will amplify into further unexpected and
                potentially hard to reproduce behaviors.  What they are and why they happen are left as
                exercise for the users and readers of this demo application.
            "</li>
        </ol>
        <script src="/highlight.min.js"></script>
        <script>"hljs.highlightAll();"</script>
    }
}

#[component]
fn NaiveEvent(
    #[prop(optional)] hook: bool,
    #[prop(optional)] fallback: bool,
) -> impl IntoView {
    let render_hook = "\
document.querySelector('#hljs-src')
    .addEventListener('load', (e) => { hljs.highlightAll() }, false);";
    let render_call = "\
if (window.hljs) {
    hljs.highlightAll();
} else {
    document.querySelector('#hljs-src')
        .addEventListener('load', (e) => { hljs.highlightAll() }, false);
}";
    let js_hook = if fallback { render_call } else { render_hook };
    let explanation = if hook {
        provide_context(CodeDemoHook {
            js_hook: js_hook.to_string(),
        });
        if fallback {
            view! {
                <ol>
                    <li>"
                        In this iteration, the following load hook is set in a "<code>"<Script>"</code>"
                        component after the dynamically loaded code example."
                        <pre><code class="language-javascript">{js_hook}</code></pre>
                    </li>
                    <li><strong>CSR</strong>"
                        This works much better now under CSR due to the fallback that checks whether the
                        library is already loaded or not.  Using the library directly if it's already loaded
                        and only register the event otherwise solves the rendering issue under CSR.
                    "</li>
                    <li><strong>SSR</strong>"
                        Much like the second example, hydration will still panic some of the time as per the
                        race condition that was described.
                    "</li>
                </ol>
                <p>"
                    All that being said, all these naive examples still result in hydration being
                    non-functional in varying degrees of (non-)reproducibility due to race conditions.  Is
                    there any way to fix this?  Is "<code>"wasm-bindgen"</code>" the only answer?  What if the
                    goal is to incorporate external scripts that change often and thus can't easily have
                    bindings built?  Follow onto the next examples to solve some of this, at the very least
                    prevent the panic during hydration.
                "</p>

            }.into_any()
        } else {
            view! {
                <ol>
                    <li>"
                        In this iteration, the following load hook is set in a "<code>"<Script>"</code>"
                        component after the dynamically loaded code example."
                        <pre><code class="language-javascript">{js_hook}</code></pre>
                    </li>
                    <li><strong>CSR</strong>"
                        Unfortunately, this still doesn't work reliably to highlight both code examples, in
                        fact, none of the code examples may highlight at all!  Placing the JavaScript loader
                        hook inside a "<code>Suspend</code>" will significantly increase the likelihood that
                        the event will be fired long before the loader adds the event hook.  As a matter of
                        fact, the highlighting is likely to only work with the largest latencies added for
                        the loading of "<code>"highlight.js"</code>", but at least both code examples will
                        highlight when working.
                    "</li>
                    <li><strong>SSR</strong>"
                        Much like the second example, hydration will still panic some of the time as per the
                        race condition that was described - basically if the timing results in CSR not showing
                        highlight code, the code will highlight here in SSR but will panic during hydration.
                    "</li>
                </ol>
            }.into_any()
        }
    } else {
        view! {
            <ol>
                <li>"
                    In this iteration, the following hook is set in a "<code>"<Script>"</code>" component
                    immediately following the one that loaded "<code>"highlight.js"</code>".
                    "<pre><code class="language-javascript">{js_hook}</code></pre>
                </li>
                <li><strong>CSR</strong>"
                    Unfortunately, the hook is being set directly on this component, rather than inside the
                    view for the dynamic block.  Given the nature of asynchronous loading which results in the
                    uncertainty of the order of events, it may or may not result in the dynamic code block (or
                    any) being highlighted under CSR (as there may or may not be a fully formed code block for
                    highlighting to happen).  This is affected by latency, so the loader here emulates a small
                    number of latency values (they repeat in a cycle).  The latency value is logged into the
                    console and it may be referred to witness its effects on what it does under CSR - look for
                    the line that might say \"loaded standard highlight.js with a minimum latency of 40 ms\".
                    Test this by going from home to here and then navigating between them using the browser's
                    back and forward feature for convenience - do ensure the "<code>"highlight.js" </code>"
                    isn't being cached by the browser.
                "</li>
                <li><strong>SSR</strong>"
                    Moreover, hydration will panic if the highlight script is loaded before hydration is
                    completed (from the resulting DOM mismatch after code highlighting).  Refreshing here
                    repeatedly may trigger the panic only some of the time when the "<code>"highlight.js"
                    </code>" script is loaded under the lowest amounts of artificial delay, as even under no
                    latency the hydration can still succeed due to the non-deterministic nature of this race
                    condition.
                "</li>
            </ol>
        }.into_any()
    };
    // FIXME Seems like <Script> require a text node, otherwise hydration error from marker mismatch
    view! {
        <h2>"Using the Leptos "<code>"<Script>"</code>" component asynchronously instead"</h2>
        <CodeDemo/>
        <Script id="hljs-src" async_="true" src="/highlight.min.js">""</Script>
        // Example 2's <Script> invocation; Example 3 and 4 will be provided via a context to allow the
        // inclusion of the `highlightAll()` call in the Suspend
        {(!hook).then(|| view! { <Script>{render_hook}</Script>})}
        <p>"
            What the "<code>"<Script>"</code>" component does is to ensure the "<code>"<script>"</code>" tag
            is placed in the document head in the order it is defined in a given component, rather than at
            where it was placed into the DOM.  Note that it is also a reactive component, much like the first
            example, it gets unloaded under CSR when the component is no longer active, In this improved
            version, "<code>"highlight.js"</code>" is also loaded asynchronously (using the "<code>"async"
            </code>" attribute), to allow an event listener that can delay highlighting to after the library
            is loaded.  This should all work out fine, right?
        "</p>
        {explanation}
    }
}

#[component]
fn CustomEvent() -> impl IntoView {
    let js_hook = format!(
        "\
var events = [];
if (!window.hljs) {{
    console.log('pushing listener for hljs load');
    events.push(new Promise((r) =>
        document.querySelector('#hljs-src').addEventListener('load', r, \
         false)));
}}
if (!window.{LEPTOS_HYDRATED}) {{
    console.log('pushing listener for leptos hydration');
    events.push(new Promise((r) => \
         document.addEventListener('{LEPTOS_HYDRATED}', r, false)));
}}
Promise.all(events).then(() => {{
    console.log(`${{events.length}} events have been dispatched; now calling \
         highlightAll()`);
    hljs.highlightAll();
}});
"
    );
    provide_context(CodeDemoHook {
        js_hook: js_hook.clone(),
    });
    // FIXME Seems like <Script> require a text node, otherwise hydration error from marker mismatch
    view! {
        <h2>"Have Leptos dispatch an event when body is hydrated"</h2>
        <CodeDemo/>
        <Script id="hljs-src" async_="true" src="/highlight.min.js">""</Script>
        <p>"
            So if using events fixes problems with timing issues, couldn't Leptos provide an event to signal
            that the body is hydrated?  Well, this problem is typically solved by having a signal in the
            component, and then inside the "<code>"Suspend"</code>" provide an "<code>"Effect"</code>" that
            would set the signal to "<code>"Some"</code>" string that will then mount the "<code>"<Script>"
            </code>" onto the body.  However, if a hydrated event is desired from within JavaScript (e.g.
            where some existing JavaScript library/framework is managing event listeners for some particular
            reason), given that typical Leptos applications provide the "<code>"fn hydate()"</code>" (usually
            in "<code>" lib.rs"</code>"), that can be achieved by providing the following after "<code>
            "leptos::mount::hydrate_body(App);"</code>".
        "</p>
        <div><pre><code class="language-rust">{format!(r#"#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {{
    use app::App;
    // ... other calls omitted, as this example is only a rough
    // reproduction of what is actually executed.
    leptos::mount::hydrate_body(App);

    // Now hydrate_body is done, provide ways to inform that
    let window = leptos::prelude::window();
    // first set a flag to signal that hydration has happened and other
    // JavaScript code may just run without waiting for the event that
    // is just about to be dispatched, as the event is only a one-time
    // deal but this lives on as a variable that can be checked.
    js_sys::Reflect::set(
        &window,
        &wasm_bindgen::JsValue::from_str({LEPTOS_HYDRATED:?}),
        &wasm_bindgen::JsValue::TRUE,
    ).expect("error setting hydrated status");
    // Then dispatch the event for all the listeners that were added.
    let event = web_sys::Event::new({LEPTOS_HYDRATED:?})
        .expect("error creating hydrated event");
    let document = leptos::prelude::document();
    document.dispatch_event(&event)
        .expect("error dispatching hydrated event");
}}"#
        )}</code></pre></div>
        <p>"
            With the notification that hydration is completed, the following JavaScript code may be called
            inside "<code>"Suspense"</code>" block (in this live example, it's triggered by providing the
            following JavaScript code via a "<code>"provide_context"</code>" which the code rendering
            component will then use within a "<code>"Suspend"</code>"):
        "</p>
        <div><pre><code class="language-javascript">{js_hook}</code></pre></div>
        <p>"
            For this simple example with a single "<code>"Suspense"</code>", no matter what latency there is,
            in whichever order the API calls are completed, the setup ensures that "<code>"highlightAll()"
            </code>" is called only after hydration is done and also after the delayed content is properly
            rendered onto the DOM.  Specifically, only use the event to wait for the required resource if it
            is not set to a ready state, and wait for all the events to become ready before actually calling
            the function.
        "</p>
        <p>"
            If there are multiple "<code>"Suspense"</code>", it will be a matter of adding all the event
            listeners that will respond to the completion of all the "<code>"Suspend"</code>"ed futures, which
            will then invoke the code highlighting function.
        "</p>
        // Leaving this last bit as a bonus page? As an exercise for the readers?
    }
}

#[component]
fn CodeDemoSignalEffect() -> impl IntoView {
    // Full JS without the use of hydration event
    // this version will unset hljs if hljs was available to throw a wrench into
    // the works, but it should still just work.
    let render_call = r#"
if (window.hljs) {
    hljs.highlightAll();
    console.log('unloading hljs to try to force the need for addEventListener for next time');
    window['hljs'] = undefined;
} else {
    document.querySelector('#hljs-src')
        .addEventListener('load', (e) => {
            hljs.highlightAll();
            console.log('using hljs inside addEventListener; leaving hljs loaded');
        }, false);
};"#;
    let code = Resource::new(|| (), |_| fetch_code());
    let (script, set_script) = signal(None::<String>);
    let code_view = move || {
        Suspend::new(async move {
            Effect::new(move |_| {
                set_script.set(Some(render_call.to_string()));
            });
            view! {
                <pre><code class="language-rust">{code.await}</code></pre>
                {
                    move || script.get().map(|script| {
                        view! { <Script>{script}</Script> }
                    })
                }
            }
        })
    };
    view! {
        <Script id="hljs-src" async_="true" src="/highlight.min.js">""</Script>
        <h2>"Using signal + effect to dynamically set "<code>"<Script>"</code>" tag as view is mounted"</h2>
        <p>"Explanation on what is being demonstrated follows after the following code example table."</p>
        <div id="code-demo">
            <table>
                <thead>
                    <tr>
                        <th>"Inline code block (part of this component)"</th>
                        <th>"Dynamic code block (loaded via server fn)"</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td><pre><code class="language-rust">{CH03_05A}</code></pre></td>
                        <td>
                            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>
                                {code_view}
                            </Suspense>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
        <p>"
            To properly ensure the "<code>"<Script>"</code>" tag containing the initialization code for the
            target JavaScript usage is executed after the "<code>"Suspend"</code>"ed view is fully rendered
            and mounted onto the DOM, with the use of an effect that sets a signal to trigger the rendering
            inside the suspend will achieve exactly that.  That was a mouthful, so let's look at the code
            for that then:
        "</p>
        <div><pre><code class="language-rust">r##"#[component]
fn CodeDemoSignalEffect() -> impl IntoView {
    let render_call = r#"
if (window.hljs) {
    hljs.highlightAll();
} else {
    document.querySelector('#hljs-src')
        .addEventListener('load', (e) => { hljs.highlightAll() }, false);
};"#;
    let code = Resource::new(|| (), |_| fetch_code());
    let (script, set_script) = signal(None::<String>);
    let code_view = move || {
        Suspend::new(async move {
            Effect::new(move |_| {
                set_script.set(Some(render_call.to_string()));
            });
            view! {
                <pre><code class="language-rust">{code.await}</code></pre>
                {
                    move || script.get().map(|script| {
                        view! { <Script>{script}</Script> }
                    })
                }
            }
        })
    };
    view! {
        <Script id="hljs-src" async_="true" src="/highlight.min.js">""</Script>
        <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>
            {code_view}
        </Suspense>
    }
}"##</code></pre></div>
        <p>"
            The "<code>"Suspend"</code>" ensures the asynchronous "<code>"Resource"</code>" will be completed
            before the view is returned, which will be mounted onto the DOM, but the initial value of the
            signal "<code>"script"</code>" will be "<code>"None"</code>", so no "<code>"<Script>"</code>" tag
            will be rendered at that stage.  Only after the suspended view is mounted onto the DOM the "<code>
            "Effect"</code>" will run, which will call "<code>"set_script"</code>" with "<code>"Some"</code>"
            value which will finally populate the "<code>"<Script>"</code>" tag with the desired JavaScript to
            be executed, in this case invoke the code highlighting feature if available otherwise wait for it.
        "</p>
        <p>"
            If there are multiple "<code>"Suspense"</code>", it will be a matter of adding the event to be
            dispatched to "<code>"set_script.set"</code>" so that it gets dispatched for the component, and
            then elsewhere above all those components a JavaScript list will tracking all the events will be
            waited on by "<code>"Promise.all"</code>", where its completion will finally invoke the desired
            JavaScript function.
        "</p>
    }
}

enum WasmDemo {
    Naive,
    ReadyEvent,
    RequestAnimationFrame,
}

#[component]
fn CodeDemoWasm(mode: WasmDemo) -> impl IntoView {
    let code = Resource::new(|| (), |_| fetch_code());
    let suspense_choice = match mode {
        WasmDemo::Naive => view! {
            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>{
                move || Suspend::new(async move {
                    view! {
                        <pre><code class="language-rust">{code.await}</code></pre>
                        {
                            #[cfg(not(feature = "ssr"))]
                            {
                                use crate::hljs::highlight_all;
                                leptos::logging::log!("calling highlight_all");
                                highlight_all();
                            }
                        }
                    }
                })
            }</Suspense>
        }.into_any(),
        WasmDemo::ReadyEvent => view! {
            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>{
                move || Suspend::new(async move {
                    view! {
                        <pre><code class="language-rust">{code.await}</code></pre>
                        {
                            #[cfg(not(feature = "ssr"))]
                            {
                                use crate::hljs;
                                use wasm_bindgen::{closure::Closure, JsCast};

                                let document = document();
                                // Rules relating to hydration still applies when loading via SSR!  Changing
                                // the dom before hydration is done is still problematic, as the same issues
                                // such as the panic as demonstrated in the relevant JavaScript demo.
                                let hydrate_listener = Closure::<dyn Fn(_)>::new(move |_: web_sys::Event| {
                                    leptos::logging::log!("wasm hydration_listener highlighting");
                                    hljs::highlight_all();
                                }).into_js_value();
                                document.add_event_listener_with_callback(
                                    LEPTOS_HYDRATED,
                                    hydrate_listener.as_ref().unchecked_ref(),
                                ).expect("failed to add event listener to document");

                                // For CSR rendering, wait for the hljs_hook which will be fired when this
                                // suspended bit is fully mounted onto the DOM, and this is done using a
                                // JavaScript shim described below.
                                let csr_listener = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                                    leptos::logging::log!("wasm csr_listener highlighting");
                                    hljs::highlight_all();
                                }).into_js_value();
                                let options = web_sys::AddEventListenerOptions::new();
                                options.set_once(true);
                                // FIXME this actually is not added as a unique function so after a quick re-
                                // render will re-add this as a new listener, which causes a double call
                                // to highlightAll.  To fix this there needs to be a way to put the listener
                                // and keep it unique, but this looks to be rather annoying to do from within
                                // this example...
                                document.add_event_listener_with_callback_and_add_event_listener_options(
                                    "hljs_hook",
                                    csr_listener.as_ref().unchecked_ref(),
                                    &options,
                                ).expect("failed to add event listener to document");
                                leptos::logging::log!("wasm csr_listener listener added");

                                // Dispatch the event when this view is finally mounted onto the DOM.
                                request_animation_frame(move || {
                                    let event = web_sys::Event::new("hljs_hook")
                                        .expect("error creating hljs_hook event");
                                    document.dispatch_event(&event)
                                        .expect("error dispatching hydrated event");
                                });
                                // Alternative, use a script tag, but at that point, you might as well write
                                // all of the above in JavaScript because in this simple example none of the
                                // above is native to Rust or Leptos.
                            }
                        }
                    }
                })
            }</Suspense>
        }.into_any(),
        WasmDemo::RequestAnimationFrame => view! {
            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>{
                move || Suspend::new(async move {
                    Effect::new(move |_| {
                        request_animation_frame(move || {
                            leptos::logging::log!("request_animation_frame invoking hljs::highlight_all");
                            // under SSR this is an noop, but it wouldn't be called under there anyway because
                            // it isn't the isomorphic version, i.e. Effect::new_isomorphic(...).
                            crate::hljs::highlight_all();
                        });
                    });
                    view! {
                        <pre><code class="language-rust">{code.await}</code></pre>
                    }
                })
            }</Suspense>
        }.into_any(),
    };
    view! {
        <p>"
            The syntax highlighting shown in the table below is done by invoking "<code>"hljs.highlightAll()"
            </code>" via the binding generated using "<code>"wasm-bindgen"</code>" - thus the ES version of "
            <code>"highlight.js"</code>" is loaded by the output bundle generated by Leptos under this set of
            demonstrations. However, things may still not work as expected, with the explanation on what is
            being demonstrated follows after the following code example table.
        "</p>
        <div id="code-demo">
            <table>
                <thead>
                    <tr>
                        <th>"Inline code block (part of this component)"</th>
                        <th>"Dynamic code block (loaded via server fn)"</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td><pre><code class="language-rust">{CH03_05A}</code></pre></td>
                        <td>{suspense_choice}</td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn WasmBindgenNaive() -> impl IntoView {
    let example = r#"<Suspense fallback=move || view! { <p>"Loading code example..."</p> }>{
    move || Suspend::new(async move {
        view! {
            <pre><code>{code.await}</code></pre>
            {
                #[cfg(not(feature = "ssr"))]
                {
                    use crate::hljs::highlight_all;
                    leptos::logging::log!("calling highlight_all");
                    highlight_all();
                }
            }
        }
    })
}</Suspense>"#;
    view! {
        <h2>"Will "<code>"wasm-bindgen"</code>" magically avoid all the problems?"</h2>
        <CodeDemoWasm mode=WasmDemo::Naive/>
        <p>"
            Well, the naively done example clearly does not work, as the behavior of this demo is almost
            exactly like the very first naive JavaScript example (after the script loaded), where only the
            inline code block will highlight under CSR and hydration is broken when trying to load this under
            SSR.  This is the consequence of porting the logic naively.  In this example, the calling of
            "<code>"hljs::highlight_all()"</code>" is located inside a "<code>"Suspend"</code>" immediately
            after the code block, but it doesn't mean the execution will apply to that because it hasn't been
            mounted onto the DOM itself for "<code>"highlight.js"</code>" to process.
        "</p>
        <p>"
            Similarly, SSR may also error under a similar mechanism, which again breaks hydration because the
            code is run on the dehydrated nodes before hydration has happened.  Using event listeners via
            "<code>"web_sys"</code>" in a similar manner like the JavaScript based solutions shown previously
            can fix this, but there are other approaches also.
        "</p>
        <p>"
            For a quick reference, the following is the "<code>"Suspense"</code>" that would ultimately render
            the dynamic code block:
        "</p>
        <div><pre><code class="language-rust">{example}</code></pre></div>
    }
}

#[component]
fn WasmBindgenJSHookReadyEvent() -> impl IntoView {
    let example = r#"#[cfg(not(feature = "ssr"))]
{
    use crate::hljs;
    use wasm_bindgen::{closure::Closure, JsCast};

    let document = document();
    // Rules relating to hydration still applies when loading via SSR!  Changing
    // the dom before hydration is done is still problematic, as the same issues
    // such as the panic as demonstrated in the relevant JavaScript demo.
    let hydrate_listener = Closure::<dyn Fn(_)>::new(move |_: web_sys::Event| {
        leptos::logging::log!("wasm hydration_listener highlighting");
        hljs::highlight_all();
    }).into_js_value();
    document.add_event_listener_with_callback(
        LEPTOS_HYDRATED,
        hydrate_listener.as_ref().unchecked_ref(),
    ).expect("failed to add event listener to document");

    // For CSR rendering, wait for the hljs_hook which will be fired when this
    // suspended bit is fully mounted onto the DOM, and this is done using a
    // JavaScript shim described below.
    let csr_listener = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
        leptos::logging::log!("wasm csr_listener highlighting");
        hljs::highlight_all();
    }).into_js_value();
    let options = web_sys::AddEventListenerOptions::new();
    options.set_once(true);
    // FIXME this actually is not added as a unique function so after a quick re-
    // render will re-add this as a new listener, which causes a double call
    // to highlightAll.  To fix this there needs to be a way to put the listener
    // and keep it unique, but this looks to be rather annoying to do from within
    // this example...
    document.add_event_listener_with_callback_and_add_event_listener_options(
        "hljs_hook",
        csr_listener.as_ref().unchecked_ref(),
        &options,
    ).expect("failed to add event listener to document");
    leptos::logging::log!("wasm csr_listener listener added");

    // Dispatch the event when this view is finally mounted onto the DOM.
    request_animation_frame(move || {
        let event = web_sys::Event::new("hljs_hook")
            .expect("error creating hljs_hook event");
        document.dispatch_event(&event)
            .expect("error dispatching hydrated event");
    });
    // Alternative, use a script tag, but at that point, you might as well write
    // all of the above in JavaScript because in this simple example none of the
    // above is native to Rust or Leptos.
}"#;

    view! {
        <h2>"Using "<code>"wasm-bindgen"</code>" with proper consideration"</h2>
        <CodeDemoWasm mode=WasmDemo::ReadyEvent/>
        <p>"
            Well, this works a lot better, under SSR the code is highlighted only after hydration to avoid the
            panic, and under CSR a new event is created for listening and responding to for the rendering to
            happen only after the suspended node is populated onto the DOM.  There is a bit of a kink with the
            way this is implemented, but it largely works.
        "</p>
        <p>"
            The code that drives this is needlessly overcomplicated, to say the least.  This is what got added
            to the "<code>"view! {...}"</code>" from the last example:
        "</p>
        <details>
            <summary>"Expand for the rather verbose code example"</summary>
            <div><pre><code class="language-rust">{example}</code></pre></div>
        </details>
        <p>"
            Given that multiple frameworks that will manipulate the DOM in their own and assume they are the
            only source of truth is the problem - being demonstrated by Leptos in previous examples assuming
            that nothing else would change the DOM for hydration.  So if it is possible to use the JavaScript
            library in a way that wouldn't cause unexpected DOM changes, then that can be a way to avoid
            needing all these additional event listeners for working around the panics.
        "</p>
        <p>"
            One thing to note is that this is a very simple example with a single Suspense (or Transition), so
            if there are more than one of them and they have significantly different resolution timings,
            calling that potentially indiscriminate JavaScript DOM manipulation function may require
            additional care (e.g. needing to wait for all the events in a future before making the final call
            to do make the invasive DOM manipulation).  Let's look at one more similar example that use a
            cheap workaround that may work for cases like integrating the simple JavaScript library here.
        "</p>
    }
}

#[component]
fn WasmBindgenEffect() -> impl IntoView {
    let example = r#"<Suspense fallback=move || view! { <p>"Loading code example..."</p> }>{
    move || Suspend::new(async move {
        Effect::new(move |_| {
            request_animation_frame(move || {
                leptos::logging::log!("request_animation_frame invoking hljs::highlight_all");
                // under SSR this is an noop.
                crate::hljs::highlight_all();
            });
        });
        view! {
            <pre><code>{code.await}</code></pre>
        }
    })
}</Suspense>"#;

    view! {
        <h2>"Using "<code>"wasm-bindgen"</code>" with proper consideration, part 2"</h2>
        <CodeDemoWasm mode=WasmDemo::RequestAnimationFrame/>
        <p>"
            This example simply uses "<code>"window.requestAnimationFrame()"</code>" (via the binding
            available as "<code>"leptos::prelude::request_animation_frame"</code>") to delay the running of
            the highlighting by a tick so that both the hydration would complete for SSR, and that it would
            also delay highlighting call to after the suspend results are loaded onto the DOM.  The Suspend
            for the dynamic code block is simply reduced to the following:
        "</p>
        <div><pre><code class="language-rust">{example}</code></pre></div>
        <p>"
            However, this method does have a drawback, which is that the inline code blocks will be processed
            multiple times by this indiscriminate method (which "<code>"highlight.js"</code>" thankfully has a
            failsafe detection which avoids issues, but definitely don't count on this being the norm with
            JavaScript libraries).  We could go back to the previous example where we use events to trigger
            for when the Suspend is resolved, but this will mean there needs to be some way to co-ordinate and
            wait for all of them to ensure the JavaScript library is only invoked once on the hydrated output.
        "</p>
        <p>"
            If the JavaScript library provides an alternative API that does not involve this wrestling of the
            DOM but does achieve the intended objectives is in fact available, it would definitely be the
            better choice.  Even better, make them available in Rust through "<code>"wasm-bindgen"</code>" so
            that the relevant Leptos component may use them directly.  In the next couple examples we will see
            how this idea may be put into practice.
        "</p>
    }
}

#[derive(Clone)]
struct InnerEffect;

#[component]
fn CodeInner(code: String, lang: String) -> impl IntoView {
    // lang is currently unused for SSR, so just drop it now to use it to avoid warning.
    #[cfg(feature = "ssr")]
    drop(lang);
    if use_context::<InnerEffect>().is_none() {
        #[cfg(feature = "ssr")]
        let inner = Some(html_escape::encode_text(&code).into_owned());
        #[cfg(not(feature = "ssr"))]
        let inner = {
            let inner = crate::hljs::highlight(code, lang);
            leptos::logging::log!(
                "about to populate inner_html with: {inner:?}"
            );
            inner
        };
        view! {
            <pre><code inner_html=inner></code></pre>
        }
        .into_any()
    } else {
        let (inner, set_inner) = signal(String::new());
        #[cfg(feature = "ssr")]
        {
            set_inner.set(html_escape::encode_text(&code).into_owned());
        };
        #[cfg(not(feature = "ssr"))]
        {
            leptos::logging::log!("calling out to hljs::highlight");
            let result = crate::hljs::highlight(code, lang);
            Effect::new(move |_| {
                leptos::logging::log!(
                    "setting the result of hljs::highlight inside an effect"
                );
                if let Some(r) = result.clone() {
                    set_inner.set(r)
                }
            });
        };
        view! {
            <pre><code inner_html=inner></code></pre>
        }
        .into_any()
    }
}

#[component]
fn CodeDemoWasmInner() -> impl IntoView {
    let code = Resource::new(|| (), |_| fetch_code());
    let code_view = move || {
        Suspend::new(async move {
            code.await.map(|code| {
                view! {
                    <CodeInner code=code lang="rust".to_string()/>
                }
            })
        })
    };
    view! {
        <p>"
            The following code examples are assigned via "<code>"inner_html"</code>" after processing through
            the relevant/available API call depending on SSR/CSR, without using any "<code>"web_sys"</code>"
            events or DOM manipulation outside of Leptos.
        "</p>
        <div id="code-demo">
            <table>
                <thead>
                    <tr>
                        <th>"Inline code block (part of this component)"</th>
                        <th>"Dynamic code block (loaded via server fn)"</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td><CodeInner code=CH03_05A.to_string() lang="rust".to_string()/></td>
                        <td>
                            <Suspense fallback=move || view! { <p>"Loading code example..."</p> }>
                                {code_view}
                            </Suspense>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn WasmBindgenDirect() -> impl IntoView {
    let code = r#"#[component]
fn CodeInner(code: String, lang: String) -> impl IntoView {
    #[cfg(feature = "ssr")]
    let inner = Some(html_escape::encode_text(&code).into_owned());
    #[cfg(not(feature = "ssr"))]
    let inner = crate::hljs::highlight(code, lang);
    view! {
        <pre><code inner_html=inner></code></pre>
    }
}

// Simply use the above component in a view like so:
//
// view! { <CodeInner code lang/> }"#
        .to_string();
    let lang = "rust".to_string();

    view! {
        <h2>"If possible, avoid DOM manipulation outside of Leptos"</h2>
        <CodeDemoWasmInner/>
        <p>"
            Whenever possible, look for a way to use the target JavaScript library to produce the desired
            markup without going through a global DOM manipulation can end up being much more straight-forward
            to write when working in pure Rust code.  More so if there is a server side counterpart, which
            means the use of the module don't need the disambiguation within the component itself.  A
            simplified version of a component that will render a code block that gets highlighted under CSR
            (and plain text under SSR) may look something like this:
        "</p>
        <CodeInner code lang/>
        <p>"
            In the above example, no additional "<code>"<script>"</code>" tags, post-hydration processing,
            event listeners nor other DOM manipuation are needed, as the JavaScript function that converts a
            string to highlighted markup can be made from Rust through bindings generated with the use of
            "<code>"wasm-bindgen"</code>" under CSR.  As the highlight functionality isn't available under
            SSR, the incoming code is simply processed using "<code>"html_escape::encode_text"</code>".
        "</p>
        <p>"
            ... Well, if only it actually works, as there is a bit of an unexpected surprise during hydration.
            During the hydration of the above code rendering component, the CSR specific pipeline kicks in and
            calls "<code>"hljs::highlight"</code>", producing a different output that was assumed to trigger
            a re-rendering.  As hydration assumes the HTML rendered under SSR is isomorphic with CSR, a
            violation of this expectation (i.e. CSR rendering something entierly different) is not something
            it anticipates; the lack of re-rendering is in fact an optimization for performance reasons as it
            avoids unnecessary work.  However in this instance, that isn't the desired behavior as the the
            syntax highlighting will not be shown as expected, and thankfully in this instance it does not
            result in a crash.
        "</p>
        <p>"
            All that being said, the code is not doing what is desired, is there any way to go about this?
            Fortunately, this is where effects comes in as it provides the intent to do something on the
            client side, being able to function as an opt-in for CSR content to \"overwrite\" SSR content.
            The next and final example will show how this should be done.
        "</p>
    }
}

#[component]
fn WasmBindgenDirectFixed() -> impl IntoView {
    let code = r#"#[component]
fn CodeInner(code: String, lang: String) -> impl IntoView {
    let (inner, set_inner) = signal(String::new());
    #[cfg(feature = "ssr")]
    {
        set_inner.set(html_escape::encode_text(&code).into_owned());
    }
    #[cfg(not(feature = "ssr"))]
    {
        let result = crate::hljs::highlight(code, lang);
        Effect::new(move |_| {
            if let Some(r) = result.clone() { set_inner.set(r) }
        });
    }
    view! {
        <pre><code inner_html=inner></code></pre>
    }
}"#
    .to_string();
    let lang = "rust".to_string();
    provide_context(InnerEffect);

    view! {
        <h2>"Corrected example using signal + effect (again)."</h2>
        <CodeDemoWasmInner/>
        <p>"
            Since the previous example didn't quite get everything working due to the component here providing
            different content between SSR and CSR, using client side signal and effect can opt-in the
            difference to overwrite the SSR rendering when hydration is complete.  This is pretty much the
            identical approach as example 5 as it is the idiomatic solution.  The improved version of the code
            rendering component from the previous example may look something like the following:
        "</p>
        <CodeInner code lang/>
        <p>"
            With the use of effects, the expected final rendering after hydration and under CSR will be the
            highlighted version as expected.  As part of trial and error, the author previously tried to
            workaround this issue by using events via "<code>"web_sys"</code>" hack around signal, but again,
            using effects like so is a lot better for this particular library.
        "</p>
        <p>"
            Given the difference between CSR and SSR, the two different renderings are disambiguated via the
            use of "<code>"[cfg(feature = ...)]"</code>" for the available behavior.  If there is a
            corresponding API to provided highlighting markup under SSR, this feature gating would be managed
            at the library level and the component would simply call the "<code>"highlight"</code>" function
            directly, resulting in both SSR/CSR rendering being fully isomorphic even with JavaScript disabled
            on the client.
        "</p>
        <p>"
            To include the output of JavaScript code for SSR may be achieved in any of the following ways:
        "</p>
        <ul>
            <li>"
                Run a JavaScript code in some JavaScript runtime such as Node.js, SpiderMonkey or Deno with
                the input, and return the collected output.
            "</li>
            <li>"
                Use a JavaScript engine as above but more directly through some kind of Rust bindings through
                packages such as "<code>"rusty_v8"</code>" or "<code>"mozjs"</code>".
            "</li>
            <li>"
                Or go the full WASM route - compile the required JavaScript into WASM and use that through
                Wasmtime on the server.
            "</li>
        </ul>
        <p>"
            All of the above are very much outside the scope of this demo which is already showing the too
            many ways to include JavaScript into a Leptos project.
        "</p>
    }
}
