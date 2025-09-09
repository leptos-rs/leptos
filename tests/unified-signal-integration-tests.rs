//! Integration tests for the Unified Signal API
//! 
//! This test suite covers real-world scenarios, edge cases, and complex
//! interactions between different signal types.

use leptos::unified_signal::{signal, Signal};
use leptos::unified_signal::signal as signal_module;
use leptos::prelude::{Get, Set, Update};
use reactive_graph::owner::Owner;
use any_spawner::Executor;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test a real-world todo application scenario
#[test]
fn test_todo_application_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Todo item structure
        #[derive(Clone, Debug, PartialEq)]
        struct TodoItem {
            id: u32,
            text: String,
            completed: bool,
        }

        // Application state
        let todos = signal(owner.clone(), Vec::<TodoItem>::new());
        let filter = signal(owner.clone(), "all".to_string());
        let new_todo_text = signal(owner.clone(), String::new());

        // Computed signals for filtered todos
        let filtered_todos = signal_module::computed(owner.clone(), {
            let todos_clone = todos.clone();
            let filter_clone = filter.clone();
            move || {
                let todos = todos_clone.get();
                let filter = filter_clone.get();
                match filter.as_str() {
                    "active" => todos.into_iter().filter(|t| !t.completed).collect(),
                    "completed" => todos.into_iter().filter(|t| t.completed).collect(),
                    _ => todos,
                }
            }
        });

        // Computed signal for todo count
        let todo_count = signal_module::computed(owner.clone(), {
            let todos_clone = todos.clone();
            move || todos_clone.get().len()
        });

        // Computed signal for completed count
        let completed_count = signal_module::computed(owner.clone(), {
            let todos_clone = todos.clone();
            move || todos_clone.get().iter().filter(|t| t.completed).count()
        });

        // Add todo function
        let add_todo = {
            let todos_clone = todos.clone();
            let new_todo_text_clone = new_todo_text.clone();
            move || {
                let text = new_todo_text_clone.get();
                if !text.is_empty() {
                    let new_todo = TodoItem {
                        id: todos_clone.get().len() as u32,
                        text: text.clone(),
                        completed: false,
                    };
                    todos_clone.update(|todos| todos.push(new_todo));
                    new_todo_text_clone.set(String::new());
                }
            }
        };

        // Toggle todo completion
        let toggle_todo = {
            let todos_clone = todos.clone();
            move |id: u32| {
                todos_clone.update(|todos| {
                    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
                        todo.completed = !todo.completed;
                    }
                });
            }
        };

        // Test initial state
        assert_eq!(todo_count.get(), 0);
        assert_eq!(completed_count.get(), 0);
        assert_eq!(filtered_todos.get().len(), 0);

        // Add some todos
        new_todo_text.set("Learn Rust".to_string());
        add_todo();
        
        new_todo_text.set("Build a web app".to_string());
        add_todo();
        
        new_todo_text.set("Deploy to production".to_string());
        add_todo();

        // Test after adding todos
        assert_eq!(todo_count.get(), 3);
        assert_eq!(completed_count.get(), 0);
        assert_eq!(filtered_todos.get().len(), 3);

        // Complete a todo
        toggle_todo(1);
        assert_eq!(completed_count.get(), 1);
        assert_eq!(filtered_todos.get().len(), 3); // All filter

        // Test active filter
        filter.set("active".to_string());
        assert_eq!(filtered_todos.get().len(), 2);

        // Test completed filter
        filter.set("completed".to_string());
        assert_eq!(filtered_todos.get().len(), 1);

        // Complete all todos
        toggle_todo(0);
        toggle_todo(2);
        assert_eq!(completed_count.get(), 3);

        // Test all completed filter
        filter.set("completed".to_string());
        assert_eq!(filtered_todos.get().len(), 3);
    });
}

/// Test a real-world shopping cart scenario
#[test]
fn test_shopping_cart_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        #[derive(Clone, Debug, PartialEq)]
        struct Product {
            id: u32,
            name: String,
            price: f64,
        }

        #[derive(Clone, Debug, PartialEq)]
        struct CartItem {
            product: Product,
            quantity: u32,
        }

        // Products catalog
        let products = signal(owner.clone(), vec![
            Product { id: 1, name: "Laptop".to_string(), price: 999.99 },
            Product { id: 2, name: "Mouse".to_string(), price: 29.99 },
            Product { id: 3, name: "Keyboard".to_string(), price: 79.99 },
        ]);

        // Shopping cart
        let cart = signal(owner.clone(), Vec::<CartItem>::new());

        // Computed signals
        let cart_total = signal_module::computed(owner.clone(), {
            let cart_clone = cart.clone();
            move || {
                cart_clone.get().iter()
                    .map(|item| item.product.price * item.quantity as f64)
                    .sum::<f64>()
            }
        });

        let cart_item_count = signal_module::computed(owner.clone(), {
            let cart_clone = cart.clone();
            move || cart_clone.get().iter().map(|item| item.quantity).sum::<u32>()
        });

        // Add to cart function
        let add_to_cart = {
            let cart_clone = cart.clone();
            let products_clone = products.clone();
            move |product_id: u32| {
                if let Some(product) = products_clone.get().iter().find(|p| p.id == product_id) {
                    cart_clone.update(|cart| {
                        if let Some(item) = cart.iter_mut().find(|item| item.product.id == product_id) {
                            item.quantity += 1;
                        } else {
                            cart.push(CartItem {
                                product: product.clone(),
                                quantity: 1,
                            });
                        }
                    });
                }
            }
        };

        // Remove from cart function
        let remove_from_cart = {
            let cart_clone = cart.clone();
            move |product_id: u32| {
                cart_clone.update(|cart| {
                    cart.retain(|item| item.product.id != product_id);
                });
            }
        };

        // Test initial state
        assert_eq!(cart_total.get(), 0.0);
        assert_eq!(cart_item_count.get(), 0);

        // Add items to cart
        add_to_cart(1); // Laptop
        add_to_cart(2); // Mouse
        add_to_cart(1); // Another laptop

        // Test cart state
        assert_eq!(cart_item_count.get(), 3);
        assert_eq!(cart_total.get(), 999.99 + 29.99 + 999.99);

        // Add more items
        add_to_cart(3); // Keyboard
        add_to_cart(2); // Another mouse

        // Test updated cart state
        assert_eq!(cart_item_count.get(), 5);
        assert_eq!(cart_total.get(), 999.99 + 29.99 + 999.99 + 79.99 + 29.99);

        // Remove an item
        remove_from_cart(2); // Remove mouse

        // Test after removal
        assert_eq!(cart_item_count.get(), 3);
        assert_eq!(cart_total.get(), 999.99 + 999.99 + 79.99);
    });
}

/// Test complex signal dependency chains
#[test]
fn test_complex_signal_dependencies() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Base signals
        let a = signal(owner.clone(), 1);
        let b = signal(owner.clone(), 2);
        let c = signal(owner.clone(), 3);

        // First level dependencies
        let sum_ab = signal_module::computed(owner.clone(), {
            let a_clone = a.clone();
            let b_clone = b.clone();
            move || a_clone.get() + b_clone.get()
        });

        let product_bc = signal_module::computed(owner.clone(), {
            let b_clone = b.clone();
            let c_clone = c.clone();
            move || b_clone.get() * c_clone.get()
        });

        // Second level dependencies
        let sum_all = signal_module::computed(owner.clone(), {
            let a_clone = a.clone();
            let sum_ab_clone = sum_ab.clone();
            let product_bc_clone = product_bc.clone();
            move || a_clone.get() + sum_ab_clone.get() + product_bc_clone.get()
        });

        // Third level dependency
        let final_result = signal_module::computed(owner.clone(), {
            let sum_all_clone = sum_all.clone();
            move || sum_all_clone.get() * 2
        });

        // Test initial values
        assert_eq!(sum_ab.get(), 3); // 1 + 2
        assert_eq!(product_bc.get(), 6); // 2 * 3
        assert_eq!(sum_all.get(), 10); // 1 + 3 + 6
        assert_eq!(final_result.get(), 20); // 10 * 2

        // Change base signal and test propagation
        a.set(5);
        assert_eq!(sum_ab.get(), 7); // 5 + 2
        assert_eq!(product_bc.get(), 6); // 2 * 3 (unchanged)
        assert_eq!(sum_all.get(), 18); // 5 + 7 + 6
        assert_eq!(final_result.get(), 36); // 18 * 2

        // Change another base signal
        b.set(4);
        assert_eq!(sum_ab.get(), 9); // 5 + 4
        assert_eq!(product_bc.get(), 12); // 4 * 3
        assert_eq!(sum_all.get(), 26); // 5 + 9 + 12
        assert_eq!(final_result.get(), 52); // 26 * 2
    });
}

/// Test signal splitting in real-world scenarios
#[test]
fn test_signal_splitting_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Create a signal and split it
        let user_data = signal(owner.clone(), UserData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        });

        let (read_user, write_user) = user_data.split();

        // Test read-only access
        let user_name = signal_module::computed(owner.clone(), {
            let read_user_clone = read_user.clone();
            move || read_user_clone.get().name.clone()
        });

        let user_email = signal_module::computed(owner.clone(), {
            let read_user_clone = read_user.clone();
            move || read_user_clone.get().email.clone()
        });

        // Test initial values
        assert_eq!(user_name.get(), "John Doe");
        assert_eq!(user_email.get(), "john@example.com");

        // Update user data
        write_user.set(UserData {
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            age: 25,
        });

        // Test updated values
        assert_eq!(user_name.get(), "Jane Smith");
        assert_eq!(user_email.get(), "jane@example.com");
    });
}

#[derive(Clone, Debug, PartialEq)]
struct UserData {
    name: String,
    email: String,
    age: u32,
}

/// Test async signals in real-world scenarios
#[test]
fn test_async_signals_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        // Simulate API calls
        let user_data = signal_module::r#async(owner.clone(), || async {
            sleep(Duration::from_millis(10)).await;
            UserData {
                name: "Async User".to_string(),
                email: "async@example.com".to_string(),
                age: 28,
            }
        });

        let posts = signal_module::r#async(owner.clone(), || async {
            sleep(Duration::from_millis(5)).await;
            vec![
                "First post".to_string(),
                "Second post".to_string(),
                "Third post".to_string(),
            ]
        });

        // Computed signal that depends on async data
        let user_summary = signal_module::computed(owner.clone(), {
            let user_data_clone = user_data.clone();
            let posts_clone = posts.clone();
            move || {
                let user = user_data_clone.get();
                let posts = posts_clone.get();
                format!("{} has {} posts", 
                    user.as_ref().map(|u| u.name.as_str()).unwrap_or("Loading..."),
                    posts.as_ref().map(|p| p.len()).unwrap_or(0)
                )
            }
        });

        // Test initial loading state
        assert_eq!(user_data.get(), None);
        assert_eq!(posts.get(), None);
        assert_eq!(user_summary.get(), "Loading... has 0 posts");

        // Note: In a real implementation, the async signals would eventually
        // resolve to Some(data), but our current implementation just returns None
        // This test demonstrates the structure for async signal integration
    });
}

/// Test edge cases with empty collections
#[test]
fn test_empty_collections_edge_cases() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let empty_vec = signal(owner.clone(), Vec::<i32>::new());
        let empty_string = signal(owner.clone(), String::new());
        let empty_option = signal(owner.clone(), Option::<i32>::None);

        // Computed signals on empty collections
        let vec_length = signal_module::computed(owner.clone(), {
            let empty_vec_clone = empty_vec.clone();
            move || empty_vec_clone.get().len()
        });

        let string_length = signal_module::computed(owner.clone(), {
            let empty_string_clone = empty_string.clone();
            move || empty_string_clone.get().len()
        });

        let option_value = signal_module::computed(owner.clone(), {
            let empty_option_clone = empty_option.clone();
            move || empty_option_clone.get().unwrap_or(0)
        });

        // Test empty states
        assert_eq!(vec_length.get(), 0);
        assert_eq!(string_length.get(), 0);
        assert_eq!(option_value.get(), 0);

        // Add data and test
        empty_vec.set(vec![1, 2, 3]);
        empty_string.set("Hello".to_string());
        empty_option.set(Some(42));

        assert_eq!(vec_length.get(), 3);
        assert_eq!(string_length.get(), 5);
        assert_eq!(option_value.get(), 42);
    });
}

/// Test signal cloning and sharing
#[test]
fn test_signal_cloning_and_sharing() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let shared_data = signal(owner.clone(), 42);
        let shared_data_clone1 = shared_data.clone();
        let shared_data_clone2 = shared_data.clone();

        // Test that all clones share the same data
        assert_eq!(shared_data.get(), 42);
        assert_eq!(shared_data_clone1.get(), 42);
        assert_eq!(shared_data_clone2.get(), 42);

        // Update through one clone
        shared_data_clone1.set(100);

        // Test that all clones see the update
        assert_eq!(shared_data.get(), 100);
        assert_eq!(shared_data_clone1.get(), 100);
        assert_eq!(shared_data_clone2.get(), 100);

        // Update through another clone
        shared_data_clone2.update(|val| *val += 50);

        // Test that all clones see the update
        assert_eq!(shared_data.get(), 150);
        assert_eq!(shared_data_clone1.get(), 150);
        assert_eq!(shared_data_clone2.get(), 150);
    });
}

/// Test Arc signals for non-Clone types
#[test]
fn test_arc_signals_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        #[derive(Debug)]
        struct NonCloneData {
            value: i32,
            name: String,
        }

        let non_clone_data = Arc::new(NonCloneData {
            value: 42,
            name: "Test".to_string(),
        });

        let arc_signal = signal_module::arc(owner.clone(), non_clone_data);

        // Test initial value
        assert_eq!(arc_signal.get().value, 42);
        assert_eq!(arc_signal.get().name, "Test");

        // Create new data and update
        let new_data = Arc::new(NonCloneData {
            value: 100,
            name: "Updated".to_string(),
        });

        arc_signal.set(new_data);

        // Test updated value
        assert_eq!(arc_signal.get().value, 100);
        assert_eq!(arc_signal.get().name, "Updated");
    });
}

/// Test complex derived signal chains
#[test]
fn test_complex_derived_chains() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let base = signal(owner.clone(), 10);

        // Create a chain of derived signals
        let doubled = base.derive(|b| *b * 2);
        let squared = doubled.derive(|d| *d * *d);
        let final_result = squared.derive(|s| *s + 1);

        // Test the chain
        assert_eq!(base.get(), 10);
        assert_eq!(doubled.get(), 20);
        assert_eq!(squared.get(), 400);
        assert_eq!(final_result.get(), 401);

        // Update base and test propagation
        base.set(5);
        assert_eq!(base.get(), 5);
        assert_eq!(doubled.get(), 10);
        assert_eq!(squared.get(), 100);
        assert_eq!(final_result.get(), 101);
    });
}

/// Test error handling in complex scenarios
#[test]
fn test_error_handling_integration() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        let data = signal(owner.clone(), vec![1, 2, 3, 4, 5]);

        // Computed signal that might cause issues
        let safe_division = signal_module::computed(owner.clone(), {
            let data_clone = data.clone();
            move || {
                let values = data_clone.get();
                if values.is_empty() {
                    0.0
                } else {
                    let sum: i32 = values.iter().sum();
                    let count = values.len() as f64;
                    sum as f64 / count
                }
            }
        });

        // Test normal operation
        assert_eq!(safe_division.get(), 3.0); // (1+2+3+4+5)/5 = 3.0

        // Test edge case - empty vector
        data.set(vec![]);
        assert_eq!(safe_division.get(), 0.0);

        // Test edge case - single element
        data.set(vec![42]);
        assert_eq!(safe_division.get(), 42.0);
    });
}

/// Test performance with many signals
#[test]
fn test_many_signals_performance() {
    _ = Executor::init_futures_executor();
    let owner = Owner::new();
    owner.with(|| {
        const SIGNAL_COUNT: usize = 1000;
        
        // Create many signals
        let mut signals = Vec::new();
        for i in 0..SIGNAL_COUNT {
            signals.push(signal(owner.clone(), i as i32));
        }

        // Create a computed signal that depends on all of them
        let sum_all = signal_module::computed(owner.clone(), {
            let signals_clone = signals.clone();
            move || {
                signals_clone.iter().map(|s| s.get()).sum::<i32>()
            }
        });

        // Test initial sum
        let expected_sum: i32 = (0..SIGNAL_COUNT as i32).sum();
        assert_eq!(sum_all.get(), expected_sum);

        // Update some signals
        signals[0].set(1000);
        signals[SIGNAL_COUNT - 1].set(2000);

        // Test updated sum
        let new_expected_sum = expected_sum - 0 - (SIGNAL_COUNT as i32 - 1) + 1000 + 2000;
        assert_eq!(sum_all.get(), new_expected_sum);
    });
}
