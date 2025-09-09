# Leptos API Reference

Complete API reference for the Leptos web framework.

## Core Framework (`leptos`)

### Reactive System

#### Signals
Basic reactive primitives for state management.

**`signal<T>(value: T) -> (ReadSignal<T>, WriteSignal<T>)`**
```rust
let (count, set_count) = signal(0);
```
Creates a signal with separate read and write handles.

**`RwSignal<T>`**
```rust
let count = RwSignal::new(0);
count.set(1);
let value = count.get();
```
Combined read/write signal handle.

#### Computations

**`Memo<T>`**
```rust
let double = Memo::new(move |_| count.get() * 2);
```
Cached derived values that update when dependencies change.

**`Resource<T>`**
```rust
let resource = Resource::new(
    move || count.get(),
    |count| async move { fetch_data(count).await }
);
```
For loading data asynchronously with reactive dependencies.

#### Effects

**`Effect::new`**
```rust
Effect::new(move |_| {
    log!("Count: {}", count.get());
});
```
Side effects that run when dependencies change.

### Component System

#### Components

**`#[component]`**
```rust
#[component]
pub fn Counter(initial_value: i32) -> impl IntoView {
    let (value, set_value) = signal(initial_value);
    view! {
        <div>
            <button on:click=move |_| set_value.update(|n| *n += 1)>
                {value}
            </button>
        </div>
    }
}
```

**Props and Children**
```rust
#[component]
pub fn Container(children: Children) -> impl IntoView {
    view! {
        <div class="container">
            {children()}
        </div>
    }
}
```

#### View Syntax

**`view!` macro**
```rust
view! {
    <div class="app">
        <h1>"Hello World"</h1>
        <Counter initial_value=0/>
        {move || if show_extra() {
            view! { <p>"Extra content"</p> }
        } else {
            view! {}
        }}
    </div>
}
```

**Event Handlers**
```rust
view! {
    <button on:click=move |_| handle_click()>
        "Click me"
    </button>
}
```

**Reactive Attributes**
```rust
view! {
    <div
        class:active=move || is_active.get()
        style:color=move || if error.get() { "red" } else { "black" }
    >
        "Content"
    </div>
}
```

### Control Flow

**`<Show/>`**
```rust
view! {
    <Show
        when=move || user.get().is_some()
        fallback=|| view! { <p>"Please log in"</p> }
    >
        <WelcomeUser/>
    </Show>
}
```

**`<For/>`**
```rust
view! {
    <For
        each=move || items.get()
        key=|item| item.id
        children=move |item| view! {
            <ItemComponent item=item/>
        }
    />
}
```

**`<Suspense/>`**
```rust
view! {
    <Suspense fallback=move || view! { <p>"Loading..."</p> }>
        <AsyncContent/>
    </Suspense>
}
```

**`<ErrorBoundary/>`**
```rust
view! {
    <ErrorBoundary
        fallback=|errors| view! {
            <div class="error">
                <p>"Something went wrong:"</p>
                <ul>
                    {move || errors.get()
                        .into_iter()
                        .map(|(_, e)| view! { <li>{e.to_string()}</li>})
                        .collect_view()
                    }
                </ul>
            </div>
        }
    >
        <MyComponent/>
    </ErrorBoundary>
}
```

## Server Functions (`server_fn`)

### `#[server]` Macro

**Basic Server Function**
```rust
#[server]
pub async fn get_posts() -> Result<Vec<Post>, ServerFnError> {
    // Server-only code
    let posts = database::get_all_posts().await?;
    Ok(posts)
}

// Usage
let posts = get_posts().await?;
```

**With Arguments**
```rust
#[server]
pub async fn create_post(title: String, content: String) -> Result<Post, ServerFnError> {
    let post = database::create_post(title, content).await?;
    Ok(post)
}
```

**With Custom Endpoint**
```rust
#[server(CreatePost, "/api/posts")]
pub async fn create_post(title: String, content: String) -> Result<Post, ServerFnError> {
    // Implementation
}
```

### Server Actions

**`ServerAction<T>`**
```rust
let create_post = ServerAction::new();

// In your view
view! {
    <form on:submit=move |ev| {
        ev.prevent_default();
        create_post.dispatch(CreatePostData { title, content });
    }>
        // Form fields
    </form>
}
```

**`ActionForm<T>`**
```rust
view! {
    <ActionForm action=create_post>
        <input type="text" name="title"/>
        <textarea name="content"></textarea>
        <button type="submit">"Create Post"</button>
    </ActionForm>
}
```

## Router (`leptos_router`)

### Route Definition

**`<Router/>`**
```rust
view! {
    <Router>
        <nav>
            <A href="/">"Home"</A>
            <A href="/about">"About"</A>
        </nav>
        <main>
            <Routes>
                <Route path="/" view=Home/>
                <Route path="/about" view=About/>
                <Route path="/users/:id" view=UserProfile/>
                <Route path="/*any" view=NotFound/>
            </Routes>
        </main>
    </Router>
}
```

### Navigation

**`<A/>` Component**
```rust
view! {
    <A href="/users/123" class="user-link">
        "View User Profile"
    </A>
}
```

**`use_navigate`**
```rust
let navigate = use_navigate();
let go_to_user = move |id: u32| {
    navigate(&format!("/users/{}", id), Default::default());
};
```

### Route Parameters

**`use_params`**
```rust
#[component]
pub fn UserProfile() -> impl IntoView {
    let params = use_params::<UserParams>();
    let user_id = move || {
        params.with(|params| {
            params.as_ref()
                .map(|p| p.id)
                .unwrap_or_default()
        })
    };
    
    view! {
        <div>"User ID: " {user_id}</div>
    }
}

#[derive(Params, PartialEq)]
struct UserParams {
    id: u32,
}
```

**`use_query`**
```rust
let query = use_query::<SearchQuery>();
let search_term = move || {
    query.with(|q| q.as_ref().ok()?.search.clone())
};
```

## Meta (`leptos_meta`)

### HTML Head Management

**`<Title/>`**
```rust
view! {
    <Title text="My App - Home"/>
}
```

**`<Meta/>`**
```rust
view! {
    <Meta name="description" content="My awesome app"/>
    <Meta property="og:title" content="My App"/>
}
```

**`<Link/>`**
```rust
view! {
    <Link rel="stylesheet" href="/styles.css"/>
}
```

**`<Script/>`**
```rust
view! {
    <Script src="/analytics.js"/>
}
```

## Reactive Graph (`reactive_graph`)

### Core Traits

**`Get` and `Set`**
```rust
use reactive_graph::traits::{Get, Set};

let signal = RwSignal::new(0);
let value = signal.get(); // Read
signal.set(42);          // Write
```

**`Update` and `Track`**
```rust
signal.update(|n| *n += 1);  // Update in place
signal.track();              // Subscribe to changes
```

### Advanced Reactivity

**`create_memo`**
```rust
let double = create_memo(move |_| count.get() * 2);
```

**`create_effect`**
```rust
create_effect(move |_| {
    log!("Count changed to: {}", count.get());
});
```

**`batch`**
```rust
batch(move || {
    count.set(10);
    name.set("John".to_string());
    // Effects run only once after batch
});
```

## Configuration (`leptos_config`)

### App Configuration

**`LeptosOptions`**
```rust
#[derive(Clone, Debug)]
pub struct LeptosOptions {
    pub output_name: String,
    pub site_root: String,
    pub site_pkg_dir: String,
    pub site_addr: SocketAddr,
    pub reload_port: u16,
    // ... other fields
}
```

**Configuration Loading**
```rust
let conf = get_configuration(None).await.unwrap();
let leptos_options = conf.leptos_options;
```

## Integration Utilities

### Axum Integration (`integrations/axum`)

**`LeptosRoutes`**
```rust
use leptos_axum::{generate_route_list, LeptosRoutes};

let routes = generate_route_list(App);

let app = Router::new()
    .leptos_routes(&leptos_options, routes, App)
    .with_state(leptos_options);
```

**Server Functions**
```rust
let app = Router::new()
    .route("/api/*fn_name", get(leptos_axum::handle_server_fns));
```

### Actix Integration (`integrations/actix`)

**Leptos Service**
```rust
use leptos_actix::{generate_route_list, LeptosRoutes};

HttpServer::new(move || {
    App::new()
        .leptos_routes(
            &leptos_options,
            generate_route_list(App),
            App,
        )
        .service(Files::new("/pkg", site_root.clone()))
})
```

## Common Patterns

### Loading States
```rust
view! {
    <Suspense fallback=move || view! { <div>"Loading..."</div> }>
        {move || match resource.get() {
            None => view! { <div>"Still loading..."</div> }.into_view(),
            Some(Err(e)) => view! { <div>"Error: " {e.to_string()}</div> }.into_view(),
            Some(Ok(data)) => view! { <DataDisplay data=data/> }.into_view(),
        }}
    </Suspense>
}
```

### Form Handling
```rust
let (name, set_name) = signal(String::new());
let (email, set_email) = signal(String::new());

let submit_action = create_server_action::<SubmitForm>();

view! {
    <ActionForm action=submit_action>
        <input
            type="text"
            name="name"
            prop:value=name
            on:input=move |ev| set_name(event_target_value(&ev))
        />
        <input
            type="email"
            name="email"
            prop:value=email
            on:input=move |ev| set_email(event_target_value(&ev))
        />
        <button type="submit">"Submit"</button>
    </ActionForm>
}
```

### Error Handling
```rust
let (error, set_error) = signal(None::<String>);

let handle_submit = move |_| {
    spawn_local(async move {
        match submit_data().await {
            Ok(_) => set_error(None),
            Err(e) => set_error(Some(e.to_string())),
        }
    });
};

view! {
    <div>
        <Show when=move || error.get().is_some()>
            <div class="error">{move || error.get()}</div>
        </Show>
        // Rest of UI
    </div>
}
```

## Feature Flags

### Core Features
- `csr` - Client-side rendering
- `ssr` - Server-side rendering
- `hydrate` - Hydration mode
- `nightly` - Nightly Rust features

### Optional Features
- `tracing` - Tracing integration
- `serde` - Serde support
- `rkyv` - Rkyv serialization
- `experimental-islands` - Islands architecture

## Performance Considerations

### Optimization Tips
- Use `Memo` for expensive computations
- Batch signal updates when possible
- Use `create_local_resource` for client-side only resources
- Implement proper key functions for `<For/>` loops
- Use `create_blocking_resource` for SSR-critical data

### Bundle Size
- Enable only necessary features
- Use dynamic imports for large dependencies
- Implement code splitting with lazy routes
- Consider using `wee_alloc` on wasm targets

---

*This reference covers the major APIs and patterns in Leptos. For complete details, see the individual crate documentation at [docs.rs](https://docs.rs/leptos/).*