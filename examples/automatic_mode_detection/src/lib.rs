//! Automatic Mode Detection Example
//!
//! This example demonstrates the new automatic mode detection system
//! that eliminates the need for manual feature flag configuration.

use leptos::*;
use leptos_compile_validator_derive::*;
use leptos_router::*;
use server_fn::*;

/// Server-only data structure
/// This will cause a compile error if used in client context
#[derive(ContextAware)]
#[leptos::server_only]
pub struct DatabaseConnection {
    pub connection_string: String,
}

impl DatabaseConnection {
    pub fn new() -> Self {
        Self {
            connection_string: "postgresql://localhost:5432/mydb".to_string(),
        }
    }
}

/// Client-only data structure
/// This will cause a compile error if used in server context
#[derive(ContextAware)]
#[leptos::client_only]
pub struct WebSocketClient {
    pub url: String,
}

impl WebSocketClient {
    pub fn new() -> Self {
        Self {
            url: "ws://localhost:8080".to_string(),
        }
    }
}

/// Universal data structure
/// This can be used in both client and server context
#[derive(ContextAware)]
#[leptos::universal]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
}

/// Server function that fetches user data
/// This automatically handles client/server context
#[server]
pub async fn get_user(id: u32) -> Result<User, ServerFnError> {
    // Server-only code block
    server_only! {
        let db = DatabaseConnection::new();
        // Simulate database query
        Ok(User {
            id,
            name: format!("User {}", id),
            email: format!("user{}@example.com", id),
        })
    }
}

/// Server function that updates user data
#[server]
pub async fn update_user(user: User) -> Result<User, ServerFnError> {
    server_only! {
        let db = DatabaseConnection::new();
        // Simulate database update
        Ok(user)
    }
}

/// Client-side component that uses WebSocket
#[component]
pub fn WebSocketComponent() -> impl IntoView {
    let (connected, set_connected) = create_signal(false);
    
    // Client-only code block
    client_only! {
        let ws_client = WebSocketClient::new();
        set_connected.set(true);
    }
    
    view! {
        <div>
            <h3>"WebSocket Status"</h3>
            <p>
                "Connected: "
                <span class:connected=move || connected.get()>
                    {move || if connected.get() { "Yes" } else { "No" }}
                </span>
            </p>
        </div>
    }
}

/// User profile component
#[component]
pub fn UserProfile(user_id: u32) -> impl IntoView {
    let user_resource = create_resource(
        move || user_id,
        |id| async move {
            get_user(id).await
        },
    );
    
    let (editing, set_editing) = create_signal(false);
    let (name, set_name) = create_signal(String::new());
    let (email, set_email) = create_signal(String::new());
    
    // Update form when user data loads
    Effect::new(move |_| {
        if let Some(Ok(user)) = user_resource.get() {
            set_name.set(user.name);
            set_email.set(user.email);
        }
    });
    
    let save_user = move |_| {
        if let Some(Ok(user)) = user_resource.get() {
            let updated_user = User {
                id: user.id,
                name: name.get(),
                email: email.get(),
            };
            spawn_local(async move {
                let _ = update_user(updated_user).await;
                set_editing.set(false);
            });
        }
    };
    
    view! {
        <div class="user-profile">
            <h2>"User Profile"</h2>
            
            <Suspense fallback=move || view! { <p>"Loading user..."</p> }>
                {move || {
                    user_resource.get().map(|result| match result {
                        Ok(user) => view! {
                            <div class="user-info">
                                <h3>{user.name}</h3>
                                <p>{user.email}</p>
                                
                                <Show when=move || !editing.get()>
                                    <button on:click=move |_| set_editing.set(true)>
                                        "Edit Profile"
                                    </button>
                                </Show>
                                
                                <Show when=move || editing.get()>
                                    <div class="edit-form">
                                        <input
                                            type="text"
                                            placeholder="Name"
                                            prop:value=move || name.get()
                                            on:input=move |ev| set_name.set(event_target_value(&ev))
                                        />
                                        <input
                                            type="email"
                                            placeholder="Email"
                                            prop:value=move || email.get()
                                            on:input=move |ev| set_email.set(event_target_value(&ev))
                                        />
                                        <button on:click=save_user>
                                            "Save"
                                        </button>
                                        <button on:click=move |_| set_editing.set(false)>
                                            "Cancel"
                                        </button>
                                    </div>
                                </Show>
                            </div>
                        }.into_view(),
                        Err(e) => view! {
                            <p class="error">"Error loading user: " {e.to_string()}</p>
                        }.into_view(),
                    })
                }}
            </Suspense>
        </div>
    }
}

/// Main app component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav>
                <A href="/">"Home"</A>
                <A href="/user/1">"User Profile"</A>
                <A href="/websocket">"WebSocket Demo"</A>
            </nav>
            
            <main>
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/user/:id" view=UserProfile/>
                    <Route path="/websocket" view=WebSocketComponent/>
                </Routes>
            </main>
        </Router>
    }
}

/// Home page component
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="home">
            <h1>"Automatic Mode Detection Example"</h1>
            <p>
                "This example demonstrates the new automatic mode detection system. "
                "Notice how we don't need to manually configure feature flags - "
                "the system automatically detects the appropriate mode based on our code patterns."
            </p>
            
            <div class="features">
                <h2>"Features Demonstrated"</h2>
                <ul>
                    <li>"Automatic mode detection (fullstack)"</li>
                    <li>"Context-aware validation"</li>
                    <li>"Server-only and client-only code blocks"</li>
                    <li>"Compile-time validation"</li>
                    <li>"Server functions with automatic context handling"</li>
                </ul>
            </div>
            
            <div class="validation-info">
                <h3>"Validation Features"</h3>
                <p>
                    "The build system automatically validates:"
                </p>
                <ul>
                    <li>"Feature flag conflicts"</li>
                    <li>"Context mismatches (server vs client code)"</li>
                    <li>"Invalid mode configurations"</li>
                    <li>"Missing required features"</li>
                </ul>
            </div>
        </div>
    }
}

/// Server function for getting app info
#[server]
pub async fn get_app_info() -> Result<String, ServerFnError> {
    server_only! {
        Ok("Automatic Mode Detection Example v1.0".to_string())
    }
}
