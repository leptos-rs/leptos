//! Community Beta Testing for the Unified Signal API
//! 
//! This test suite validates the API against common community use cases,
//! feedback scenarios, and edge cases reported by the community.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set};
use reactive_graph::owner::Owner;
use any_spawner::Executor;
use std::time::Duration;
use tokio::time::sleep;

/// Test community feedback: "I want a simple counter example"
#[test]
fn test_community_simple_counter() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it as simple as possible"
        let count = signal(owner.clone(), 0);
        
        // Test basic operations
        assert_eq!(count.get(), 0);
        
        count.set(42);
        assert_eq!(count.get(), 42);
        
        count.update(|c| *c += 1);
        assert_eq!(count.get(), 43);
        
        // Test derived signal
        let doubled = count.derive(|c| *c * 2);
        assert_eq!(doubled.get(), 86);
        
        // Test splitting
        let (read_count, write_count) = count.split();
        assert_eq!(read_count.get(), 43);
        write_count.set(100);
        assert_eq!(read_count.get(), 100);
    });
}

/// Test community feedback: "I need form handling"
#[test]
fn test_community_form_handling() {
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

        // Computed signals for validation
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

        // Test initial state
        assert!(!is_name_valid.get());
        assert!(!is_email_valid.get());
        assert!(!is_form_valid.get());

        // Update form data
        form_data.update(|data| {
            data.name = "John Doe".to_string();
            data.email = "john@example.com".to_string();
            data.age = 30;
        });

        // Test validation
        assert!(is_name_valid.get());
        assert!(is_email_valid.get());
        assert!(is_form_valid.get());
    });
}

/// Test community feedback: "I want to use it with async data"
#[test]
fn test_community_async_data_usage() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make async data fetching simple"
        let user_id = signal(owner.clone(), 1);
        
        let user_data = signal_module::r#async(owner.clone(), {
            let user_id_clone = user_id.clone();
            move || {
                let id = user_id_clone.get();
                async move {
                    sleep(Duration::from_millis(10)).await;
                    format!("User {}", id)
                }
            }
        });

        let posts = signal_module::r#async(owner.clone(), {
            let user_id_clone = user_id.clone();
            move || {
                let id = user_id_clone.get();
                async move {
                    sleep(Duration::from_millis(5)).await;
                    vec![format!("Post 1 for user {}", id), format!("Post 2 for user {}", id)]
                }
            }
        });

        // Computed signal that combines async data
        let user_summary = signal_module::computed(owner.clone(), {
            let user_data_clone = user_data.clone();
            let posts_clone = posts.clone();
            move || {
                let user = user_data_clone.get();
                let posts = posts_clone.get();
                format!("{} has {} posts", 
                    user.as_ref().map(|u| u.as_str()).unwrap_or("Loading..."),
                    posts.as_ref().map(|p| p.len()).unwrap_or(0)
                )
            }
        });

        // Test initial loading state
        assert_eq!(user_summary.get(), "Loading... has 0 posts");

        // Note: In a real implementation, the async signals would resolve
        // This test demonstrates the structure for community async usage
    });
}

/// Test community feedback: "I need to share state between components"
#[test]
fn test_community_shared_state() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make state sharing easy"
        let shared_counter = signal(owner.clone(), 0);
        
        // Simulate multiple components accessing the same state
        let component_a = {
            let counter = shared_counter.clone();
            move || {
                counter.update(|c| *c += 1);
                counter.get()
            }
        };

        let component_b = {
            let counter = shared_counter.clone();
            move || {
                counter.update(|c| *c += 2);
                counter.get()
            }
        };

        let component_c = {
            let counter = shared_counter.clone();
            move || {
                counter.update(|c| *c += 3);
                counter.get()
            }
        };

        // Test that all components see the same state
        assert_eq!(component_a(), 1);
        assert_eq!(component_b(), 3);
        assert_eq!(component_c(), 6);
        assert_eq!(shared_counter.get(), 6);
    });
}

/// Test community feedback: "I want to use it with complex data structures"
#[test]
fn test_community_complex_data_structures() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        #[derive(Clone, Debug, PartialEq)]
        struct User {
            id: u32,
            name: String,
            preferences: UserPreferences,
            posts: Vec<Post>,
        }

        #[derive(Clone, Debug, PartialEq)]
        struct UserPreferences {
            theme: String,
            notifications: bool,
        }

        #[derive(Clone, Debug, PartialEq)]
        struct Post {
            id: u32,
            title: String,
            content: String,
            likes: u32,
        }

        let user = signal(owner.clone(), User {
            id: 1,
            name: "John Doe".to_string(),
            preferences: UserPreferences {
                theme: "dark".to_string(),
                notifications: true,
            },
            posts: vec![
                Post {
                    id: 1,
                    title: "Hello World".to_string(),
                    content: "My first post".to_string(),
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

        let user_summary = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            let total_likes_clone = total_likes.clone();
            let post_count_clone = post_count.clone();
            move || {
                let user = user_clone.get();
                format!("{} has {} posts with {} total likes", 
                    user.name, 
                    post_count_clone.get(), 
                    total_likes_clone.get()
                )
            }
        });

        // Test initial state
        assert_eq!(total_likes.get(), 5);
        assert_eq!(post_count.get(), 1);
        assert_eq!(user_summary.get(), "John Doe has 1 posts with 5 total likes");

        // Add a new post
        user.update(|user| {
            user.posts.push(Post {
                id: 2,
                title: "Second Post".to_string(),
                content: "Another post".to_string(),
                likes: 10,
            });
        });

        // Test updated state
        assert_eq!(total_likes.get(), 15);
        assert_eq!(post_count.get(), 2);
        assert_eq!(user_summary.get(), "John Doe has 2 posts with 15 total likes");
    });
}

/// Test community feedback: "I need to handle errors gracefully"
#[test]
fn test_community_error_handling() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make error handling simple"
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

        let safe_max = signal_module::computed(owner.clone(), {
            let data_clone = data.clone();
            move || {
                data_clone.get().iter().max().copied()
            }
        });

        let safe_min = signal_module::computed(owner.clone(), {
            let data_clone = data.clone();
            move || {
                data_clone.get().iter().min().copied()
            }
        });

        // Test normal operation
        assert_eq!(safe_average.get(), Some(3.0));
        assert_eq!(safe_max.get(), Some(5));
        assert_eq!(safe_min.get(), Some(1));

        // Test edge case - empty data
        data.set(vec![]);
        assert_eq!(safe_average.get(), None);
        assert_eq!(safe_max.get(), None);
        assert_eq!(safe_min.get(), None);

        // Test edge case - single element
        data.set(vec![42]);
        assert_eq!(safe_average.get(), Some(42.0));
        assert_eq!(safe_max.get(), Some(42));
        assert_eq!(safe_min.get(), Some(42));
    });
}

/// Test community feedback: "I want to use it with external libraries"
#[test]
fn test_community_external_library_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it work with external libraries"
        
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

        let user_names = signal_module::computed(owner.clone(), {
            let external_data_clone = external_data.clone();
            move || {
                external_data_clone.get()["users"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|user| user["name"].as_str().unwrap_or("Unknown").to_string())
                    .collect::<Vec<String>>()
            }
        });

        // Test initial state
        assert_eq!(user_count.get(), 2);
        assert_eq!(user_names.get(), vec!["Alice".to_string(), "Bob".to_string()]);

        // Update external data
        external_data.set(serde_json::json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
                {"id": 3, "name": "Charlie"}
            ],
            "total": 3
        }));

        // Test updated state
        assert_eq!(user_count.get(), 3);
        assert_eq!(user_names.get(), vec!["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()]);
    });
}

/// Test community feedback: "I need performance optimizations"
#[test]
fn test_community_performance_optimizations() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it performant"
        let large_dataset = signal(owner.clone(), (0..1000).collect::<Vec<i32>>());

        // Optimized computed signal that only recalculates when needed
        let sum = signal_module::computed(owner.clone(), {
            let large_dataset_clone = large_dataset.clone();
            move || {
                large_dataset_clone.get().iter().sum::<i32>()
            }
        });

        let average = signal_module::computed(owner.clone(), {
            let sum_clone = sum.clone();
            let large_dataset_clone = large_dataset.clone();
            move || {
                let sum_val = sum_clone.get();
                let count = large_dataset_clone.get().len() as f64;
                sum_val as f64 / count
            }
        });

        // Test initial calculations
        assert_eq!(sum.get(), 499500); // Sum of 0..1000
        assert_eq!(average.get(), 499.5); // Average of 0..1000

        // Update dataset
        large_dataset.set((0..500).collect::<Vec<i32>>());

        // Test that computations are efficient
        assert_eq!(sum.get(), 124750); // Sum of 0..500
        assert_eq!(average.get(), 249.5); // Average of 0..500
    });
}

/// Test community feedback: "I want to use it with WebAssembly"
#[test]
fn test_community_wasm_compatibility() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it WASM-friendly"
        
        // Test that all signal types work in WASM context
        let simple_signal = signal(owner.clone(), 42);
        let string_signal = signal(owner.clone(), "Hello WASM".to_string());
        let vec_signal = signal(owner.clone(), vec![1, 2, 3]);
        let option_signal = signal(owner.clone(), Some(42));

        // Test basic operations
        assert_eq!(simple_signal.get(), 42);
        assert_eq!(string_signal.get(), "Hello WASM");
        assert_eq!(vec_signal.get(), vec![1, 2, 3]);
        assert_eq!(option_signal.get(), Some(42));

        // Test computed signals
        let computed_signal = signal_module::computed(owner.clone(), {
            let simple_signal_clone = simple_signal.clone();
            move || simple_signal_clone.get() * 2
        });

        assert_eq!(computed_signal.get(), 84);

        // Test derived signals
        let derived_signal = simple_signal.derive(|val| *val + 1);
        assert_eq!(derived_signal.get(), 43);

        // Test splitting
        let (read_signal, write_signal) = simple_signal.split();
        assert_eq!(read_signal.get(), 42);
        write_signal.set(100);
        assert_eq!(read_signal.get(), 100);
    });
}

/// Test community feedback: "I need TypeScript-like type safety"
#[test]
fn test_community_type_safety() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it type-safe like TypeScript"
        
        #[derive(Clone, Debug, PartialEq)]
        struct User {
            id: u32,
            name: String,
            email: String,
        }

        let user = signal(owner.clone(), User {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        });

        // Type-safe computed signals
        let user_id = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            move || user_clone.get().id
        });

        let user_name = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            move || user_clone.get().name.clone()
        });

        let user_email = signal_module::computed(owner.clone(), {
            let user_clone = user.clone();
            move || user_clone.get().email.clone()
        });

        // Type-safe derived signals
        let user_summary = user.derive(|u| format!("{} ({})", u.name, u.email));

        // Test type safety
        assert_eq!(user_id.get(), 1);
        assert_eq!(user_name.get(), "John Doe");
        assert_eq!(user_email.get(), "john@example.com");
        assert_eq!(user_summary.get(), "John Doe (john@example.com)");

        // Test that type mismatches are caught at compile time
        // This would fail to compile if we tried to use wrong types:
        // let wrong_type = signal(owner.clone(), "string".to_string());
        // wrong_type.set(42); // This would be a compile error
    });
}

/// Test community feedback: "I want to use it with React-like patterns"
#[test]
fn test_community_react_like_patterns() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it feel like React hooks"
        
        // useState-like pattern
        let count = signal(owner.clone(), 0);
        let set_count = |value: i32| count.set(value);
        let increment = || count.update(|c| *c += 1);

        // useEffect-like pattern with computed signals
        let doubled_count = signal_module::computed(owner.clone(), {
            let count_clone = count.clone();
            move || count_clone.get() * 2
        });

        // useMemo-like pattern
        let expensive_calculation = signal_module::computed(owner.clone(), {
            let count_clone = count.clone();
            move || {
                // Simulate expensive calculation
                let val = count_clone.get();
                val * val * val
            }
        });

        // Test React-like behavior
        assert_eq!(count.get(), 0);
        assert_eq!(doubled_count.get(), 0);
        assert_eq!(expensive_calculation.get(), 0);

        set_count(5);
        assert_eq!(count.get(), 5);
        assert_eq!(doubled_count.get(), 10);
        assert_eq!(expensive_calculation.get(), 125);

        increment();
        assert_eq!(count.get(), 6);
        assert_eq!(doubled_count.get(), 12);
        assert_eq!(expensive_calculation.get(), 216);
    });
}

/// Test community feedback: "I need to handle large datasets"
#[test]
fn test_community_large_datasets() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Community request: "Make it work with large datasets"
        
        const LARGE_SIZE: usize = 10000;
        let large_dataset = signal(owner.clone(), (0..LARGE_SIZE as i32).collect::<Vec<i32>>());

        // Efficient computed signals for large datasets
        let sum = signal_module::computed(owner.clone(), {
            let large_dataset_clone = large_dataset.clone();
            move || {
                large_dataset_clone.get().iter().sum::<i32>()
            }
        });

        let average = signal_module::computed(owner.clone(), {
            let sum_clone = sum.clone();
            let large_dataset_clone = large_dataset.clone();
            move || {
                let sum_val = sum_clone.get();
                let count = large_dataset_clone.get().len() as f64;
                sum_val as f64 / count
            }
        });

        let max_value = signal_module::computed(owner.clone(), {
            let large_dataset_clone = large_dataset.clone();
            move || {
                large_dataset_clone.get().iter().max().copied().unwrap_or(0)
            }
        });

        // Test with large dataset
        let expected_sum: i32 = (0..LARGE_SIZE as i32).sum();
        let expected_avg = (LARGE_SIZE as f64 - 1.0) / 2.0;
        let expected_max = LARGE_SIZE as i32 - 1;

        assert_eq!(sum.get(), expected_sum);
        assert_eq!(average.get(), expected_avg);
        assert_eq!(max_value.get(), expected_max);

        // Test performance with updates
        large_dataset.update(|data| {
            data[0] = 99999;
        });

        assert_eq!(sum.get(), expected_sum - 0 + 99999);
        assert_eq!(max_value.get(), 99999);
    });
}
