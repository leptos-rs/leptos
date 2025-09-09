//! SSR Safety Tests for the Unified Signal API
//! 
//! This test suite validates that the unified signal API handles SSR scenarios
//! correctly, preventing the signal access warnings seen in applications like CloudShuttle.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set, GetUntracked};
use reactive_graph::owner::Owner;
use any_spawner::Executor;

/// Test SSR-safe signal access with get_untracked
#[test]
fn test_ssr_safe_signal_access() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let count = signal(owner.clone(), 42);
        let name = signal(owner.clone(), "Leptos".to_string());
        
        // Test normal reactive access
        assert_eq!(count.get(), 42);
        assert_eq!(name.get(), "Leptos");
        
        // Test SSR-safe untracked access
        assert_eq!(count.get_untracked(), 42);
        assert_eq!(name.get_untracked(), "Leptos");
        
        // Update values
        count.set(100);
        name.set("Unified API".to_string());
        
        // Both access methods should see the updated values
        assert_eq!(count.get(), 100);
        assert_eq!(count.get_untracked(), 100);
        assert_eq!(name.get(), "Unified API");
        assert_eq!(name.get_untracked(), "Unified API");
    });
}

/// Test SSR-safe access with derived signals
#[test]
fn test_ssr_safe_derived_signals() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let base = signal(owner.clone(), 10);
        let doubled = base.derive(|val| *val * 2);
        let squared = doubled.derive(|val| *val * *val);
        
        // Test reactive access
        assert_eq!(base.get(), 10);
        assert_eq!(doubled.get(), 20);
        assert_eq!(squared.get(), 400);
        
        // Test SSR-safe untracked access
        assert_eq!(base.get_untracked(), 10);
        assert_eq!(doubled.get_untracked(), 20);
        assert_eq!(squared.get_untracked(), 400);
        
        // Update base value
        base.set(5);
        
        // Both access methods should see updated values
        assert_eq!(base.get(), 5);
        assert_eq!(base.get_untracked(), 5);
        assert_eq!(doubled.get(), 10);
        assert_eq!(doubled.get_untracked(), 10);
        assert_eq!(squared.get(), 100);
        assert_eq!(squared.get_untracked(), 100);
    });
}

/// Test SSR-safe access with computed signals
#[test]
fn test_ssr_safe_computed_signals() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let a = signal(owner.clone(), 1);
        let b = signal(owner.clone(), 2);
        let c = signal(owner.clone(), 3);
        
        let sum = signal_module::computed(owner.clone(), {
            let a_clone = a.clone();
            let b_clone = b.clone();
            let c_clone = c.clone();
            move || a_clone.get() + b_clone.get() + c_clone.get()
        });
        
        // Test reactive access
        assert_eq!(sum.get(), 6);
        
        // Test SSR-safe untracked access
        assert_eq!(sum.get_untracked(), 6);
        
        // Update values
        a.set(10);
        
        // Both access methods should see updated values
        assert_eq!(sum.get(), 15);
        assert_eq!(sum.get_untracked(), 15);
    });
}

/// Test SSR-safe access in routing scenarios (like CloudShuttle)
#[test]
fn test_ssr_safe_routing() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Simulate routing state
        let current_path = signal(owner.clone(), "/home".to_string());
        let user_id = signal(owner.clone(), Some(123));
        
        // Computed signals for routing logic
        let is_home = signal_module::computed(owner.clone(), {
            let current_path_clone = current_path.clone();
            move || current_path_clone.get() == "/home"
        });
        
        let is_user_page = signal_module::computed(owner.clone(), {
            let current_path_clone = current_path.clone();
            let user_id_clone = user_id.clone();
            move || {
                current_path_clone.get().starts_with("/user/") && user_id_clone.get().is_some()
            }
        });
        
        // Test reactive access (for client-side rendering)
        assert!(is_home.get());
        assert!(!is_user_page.get());
        
        // Test SSR-safe untracked access (for server-side rendering)
        assert!(is_home.get_untracked());
        assert!(!is_user_page.get_untracked());
        
        // Simulate navigation
        current_path.set("/user/123".to_string());
        
        // Both access methods should see updated values
        assert!(!is_home.get());
        assert!(!is_home.get_untracked());
        assert!(is_user_page.get());
        assert!(is_user_page.get_untracked());
    });
}

/// Test SSR-safe access with form handling
#[test]
fn test_ssr_safe_form_handling() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        #[derive(Clone, Debug, PartialEq)]
        struct FormData {
            name: String,
            email: String,
            age: u32,
        }
        
        let form_data = signal(owner.clone(), FormData {
            name: String::new(),
            email: String::new(),
            age: 0,
        });
        
        // Validation signals
        let is_name_valid = signal_module::computed(owner.clone(), {
            let form_data_clone = form_data.clone();
            move || !form_data_clone.get().name.is_empty()
        });
        
        let is_email_valid = signal_module::computed(owner.clone(), {
            let form_data_clone = form_data.clone();
            move || form_data_clone.get().email.contains('@')
        });
        
        let is_form_valid = signal_module::computed(owner.clone(), {
            let is_name_valid_clone = is_name_valid.clone();
            let is_email_valid_clone = is_email_valid.clone();
            move || is_name_valid_clone.get() && is_email_valid_clone.get()
        });
        
        // Test initial state with both access methods
        assert!(!is_name_valid.get());
        assert!(!is_name_valid.get_untracked());
        assert!(!is_email_valid.get());
        assert!(!is_email_valid.get_untracked());
        assert!(!is_form_valid.get());
        assert!(!is_form_valid.get_untracked());
        
        // Update form data
        form_data.update(|data| {
            data.name = "John Doe".to_string();
            data.email = "john@example.com".to_string();
            data.age = 30;
        });
        
        // Test updated state with both access methods
        assert!(is_name_valid.get());
        assert!(is_name_valid.get_untracked());
        assert!(is_email_valid.get());
        assert!(is_email_valid.get_untracked());
        assert!(is_form_valid.get());
        assert!(is_form_valid.get_untracked());
    });
}

/// Test SSR-safe access with async signals
#[test]
fn test_ssr_safe_async_signals() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let user_id = signal(owner.clone(), 1);
        
        // Simulate async signal (placeholder implementation)
        let user_data = signal_module::r#async(owner.clone(), {
            let user_id_clone = user_id.clone();
            move || {
                let id = user_id_clone.get();
                async move {
                    // Simulate async operation
                    format!("User {}", id)
                }
            }
        });
        
        // Test both access methods (current implementation returns None)
        assert_eq!(user_data.get(), None);
        assert_eq!(user_data.get_untracked(), None);
        
        // Note: In a real implementation, the async signal would eventually
        // resolve to Some(data), and both get() and get_untracked() would
        // return the same value, but get_untracked() would be safe for SSR
    });
}

/// Test SSR-safe access with complex nested data
#[test]
fn test_ssr_safe_complex_data() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        #[derive(Clone, Debug, PartialEq)]
        struct User {
            id: u32,
            name: String,
            posts: Vec<Post>,
        }
        
        #[derive(Clone, Debug, PartialEq)]
        struct Post {
            id: u32,
            title: String,
            likes: u32,
        }
        
        let user = signal(owner.clone(), User {
            id: 1,
            name: "John Doe".to_string(),
            posts: vec![
                Post {
                    id: 1,
                    title: "Hello World".to_string(),
                    likes: 5,
                },
            ],
        });
        
        // Computed signals for complex data
        let total_likes = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            move || {
                user_clone.get().posts.iter().map(|post| post.likes).sum::<u32>()
            }
        });
        
        let post_count = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            move || user_clone.get().posts.len()
        });
        
        // Test both access methods
        assert_eq!(total_likes.get(), 5);
        assert_eq!(total_likes.get_untracked(), 5);
        assert_eq!(post_count.get(), 1);
        assert_eq!(post_count.get_untracked(), 1);
        
        // Update user data
        user.update(|user| {
            user.posts.push(Post {
                id: 2,
                title: "Second Post".to_string(),
                likes: 10,
            });
        });
        
        // Test updated state with both access methods
        assert_eq!(total_likes.get(), 15);
        assert_eq!(total_likes.get_untracked(), 15);
        assert_eq!(post_count.get(), 2);
        assert_eq!(post_count.get_untracked(), 2);
    });
}

/// Test SSR-safe access with signal splitting
#[test]
fn test_ssr_safe_signal_splitting() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let user_data = signal(owner.clone(), UserData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        });
        
        let (read_user, write_user) = user_data.split();
        
        // Test both access methods on read signal
        assert_eq!(read_user.get().name, "John Doe");
        assert_eq!(GetUntracked::get_untracked(&read_user).name, "John Doe");
        
        // Update through write signal
        write_user.set(UserData {
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            age: 25,
        });
        
        // Test updated state with both access methods
        assert_eq!(read_user.get().name, "Jane Smith");
        assert_eq!(GetUntracked::get_untracked(&read_user).name, "Jane Smith");
    });
}

#[derive(Clone, Debug, PartialEq)]
struct UserData {
    name: String,
    email: String,
    age: u32,
}

/// Test SSR-safe access with error handling
#[test]
fn test_ssr_safe_error_handling() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let data = signal(owner.clone(), vec![1, 2, 3, 4, 5]);
        
        // Safe operations that handle errors
        let safe_average = signal_module::computed(owner.clone(), {
            let data_clone = data.clone();
            move || {
                let values = data_clone.get();
                if values.is_empty() {
                    None
                } else {
                    let sum: i32 = values.iter().sum();
                    let count = values.len() as f64;
                    Some(sum as f64 / count)
                }
            }
        });
        
        // Test both access methods
        assert_eq!(safe_average.get(), Some(3.0));
        assert_eq!(safe_average.get_untracked(), Some(3.0));
        
        // Test edge case - empty data
        data.set(vec![]);
        assert_eq!(safe_average.get(), None);
        assert_eq!(safe_average.get_untracked(), None);
    });
}

/// Test SSR-safe access with external library integration
#[test]
fn test_ssr_safe_external_library() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Simulate external library data
        let external_data = signal(owner.clone(), serde_json::json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "total": 2
        }));
        
        // Computed signals that work with external data
        let user_count = signal_module::computed(owner.clone(), {
            let external_data_clone = external_data.clone();
            move || {
                external_data_clone.get()["total"].as_u64().unwrap_or(0) as usize
            }
        });
        
        // Test both access methods
        assert_eq!(user_count.get(), 2);
        assert_eq!(user_count.get_untracked(), 2);
        
        // Update external data
        external_data.set(serde_json::json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
                {"id": 3, "name": "Charlie"}
            ],
            "total": 3
        }));
        
        // Test updated state with both access methods
        assert_eq!(user_count.get(), 3);
        assert_eq!(user_count.get_untracked(), 3);
    });
}
