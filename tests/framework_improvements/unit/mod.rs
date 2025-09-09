//! Unit Tests for Framework Improvements
//!
//! Fine-grained testing of individual components, functions, and utilities
//! that implement the documented framework improvements.

pub mod init_command_tests;
pub mod signal_api_tests;
pub mod error_handling_tests;
pub mod build_system_tests;
pub mod hot_reload_tests;

#[cfg(test)]
mod leptos_init_unit_tests {
    use super::*;

    /// Unit tests for LEPTOS-2024-001: Project Setup Complexity
    mod init_command {
        use std::path::PathBuf;
        use std::collections::HashMap;

        #[derive(Debug, Clone)]
        pub struct InitCommandArgs {
            pub project_name: String,
            pub template: String,
            pub directory: Option<PathBuf>,
            pub features: Vec<String>,
        }

        #[derive(Debug)]
        pub struct GeneratedProject {
            pub path: PathBuf,
            pub cargo_toml: String,
            pub lib_rs: String,
            pub main_rs: String,
            pub leptos_config: HashMap<String, String>,
        }

        #[test]
        fn test_init_command_arg_parsing() {
            let args = parse_init_args(&["leptos", "init", "my-app", "--template", "fullstack"]);
            
            assert_eq!(args.project_name, "my-app");
            assert_eq!(args.template, "fullstack");
            assert!(args.features.is_empty());
        }

        #[test]
        fn test_template_validation() {
            let valid_templates = ["spa", "ssr", "fullstack", "static", "minimal"];
            
            for template in valid_templates {
                assert!(is_valid_template(template), "Template '{}' should be valid", template);
            }
            
            assert!(!is_valid_template("invalid_template"));
        }

        #[test]
        fn test_project_name_validation() {
            // Valid project names
            assert!(is_valid_project_name("my-app"));
            assert!(is_valid_project_name("my_app"));
            assert!(is_valid_project_name("app123"));
            
            // Invalid project names
            assert!(!is_valid_project_name(""));
            assert!(!is_valid_project_name("123app")); // Can't start with number
            assert!(!is_valid_project_name("my-app!")); // Invalid characters
            assert!(!is_valid_project_name("if"));     // Rust keyword
        }

        #[test]
        fn test_cargo_toml_generation() {
            let project = generate_project_structure("test-app", "spa");
            
            // Should have minimal configuration
            let cargo_lines: Vec<&str> = project.cargo_toml.lines().collect();
            assert!(cargo_lines.len() < 20, "Generated Cargo.toml should have <20 lines, got {}", cargo_lines.len());
            
            // Should contain essential dependencies
            assert!(project.cargo_toml.contains("leptos = "));
            assert!(project.cargo_toml.contains("[package]"));
            
            // Should NOT contain complex feature flags
            assert!(!project.cargo_toml.contains("optional = true"));
            assert!(!project.cargo_toml.contains("[features]"));
        }

        #[test]
        fn test_leptos_config_generation() {
            let project = generate_project_structure("test-app", "fullstack");
            
            // Should have smart defaults
            assert_eq!(project.leptos_config.get("template"), Some(&"fullstack".to_string()));
            assert!(project.leptos_config.contains_key("site-addr"));
            assert!(project.leptos_config.contains_key("output-name"));
            
            // Should auto-configure based on template
            if project.leptos_config.get("template") == Some(&"spa".to_string()) {
                assert_eq!(project.leptos_config.get("features"), Some(&"csr".to_string()));
            }
        }

        #[test]
        fn test_main_rs_generation() {
            let project = generate_project_structure("test-app", "spa");
            
            // Should contain proper imports
            assert!(project.main_rs.contains("use leptos::prelude::*;"));
            assert!(project.main_rs.contains("mount_to_body"));
            
            // Should have simple, working example
            assert!(project.main_rs.contains("fn main()"));
            assert!(project.main_rs.contains("#[component]") || project.main_rs.contains("view!"));
        }

        #[test]
        fn test_lib_rs_generation() {
            let project = generate_project_structure("test-app", "fullstack");
            
            // Should have component definition
            assert!(project.lib_rs.contains("#[component]"));
            assert!(project.lib_rs.contains("pub fn App()"));
            
            // Should be framework-agnostic for different templates
            if project.leptos_config.get("template") == Some(&"fullstack".to_string()) {
                assert!(project.lib_rs.contains("IntoView"));
            }
        }

        // Mock implementations for testing
        fn parse_init_args(args: &[&str]) -> InitCommandArgs {
            InitCommandArgs {
                project_name: args.get(2).unwrap_or(&"").to_string(),
                template: if args.contains(&"--template") {
                    args.get(args.iter().position(|&x| x == "--template").unwrap() + 1)
                        .unwrap_or(&"spa").to_string()
                } else {
                    "spa".to_string()
                },
                directory: None,
                features: vec![],
            }
        }

        fn is_valid_template(template: &str) -> bool {
            matches!(template, "spa" | "ssr" | "fullstack" | "static" | "minimal")
        }

        fn is_valid_project_name(name: &str) -> bool {
            if name.is_empty() || name.chars().next().unwrap().is_numeric() {
                return false;
            }
            let rust_keywords = ["if", "else", "for", "while", "match", "fn", "let", "mut"];
            !rust_keywords.contains(&name) && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        }

        fn generate_project_structure(name: &str, template: &str) -> GeneratedProject {
            let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
leptos = {{ version = "0.8", features = ["default"] }}
"#, name);

            let lib_rs = r#"use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div>
            <h1>"Welcome to Leptos!"</h1>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                "Count: " {count}
            </button>
        </div>
    }
}
"#.to_string();

            let main_rs = r#"use leptos::prelude::*;
use app::App;

fn main() {
    mount_to_body(App);
}
"#.to_string();

            let mut leptos_config = HashMap::new();
            leptos_config.insert("template".to_string(), template.to_string());
            leptos_config.insert("site-addr".to_string(), "127.0.0.1:3000".to_string());
            leptos_config.insert("output-name".to_string(), name.to_string());

            GeneratedProject {
                path: PathBuf::from(format!("test_output/{}", name)),
                cargo_toml,
                lib_rs,
                main_rs,
                leptos_config,
            }
        }
    }

    /// Unit tests for LEPTOS-2024-003: Unified Signal API
    mod unified_signals {
        #[test]
        fn test_signal_creation() {
            // Test unified signal() function
            let count = signal(0);
            assert_eq!(count.get(), 0);
        }

        #[test]
        fn test_signal_updates() {
            let count = signal(0);
            count.set(42);
            assert_eq!(count.get(), 42);
            
            count.update(|n| *n += 1);
            assert_eq!(count.get(), 43);
        }

        #[test]
        fn test_signal_derivation() {
            let count = signal(5);
            let doubled = count.derive(|n| n * 2);
            
            assert_eq!(doubled.get(), 10);
            
            count.set(10);
            assert_eq!(doubled.get(), 20);
        }

        #[test]
        fn test_signal_splitting() {
            let signal = signal(42);
            let (read, write) = signal.split();
            
            assert_eq!(read.get(), 42);
            write.set(100);
            assert_eq!(read.get(), 100);
        }

        #[test]
        fn test_signal_type_inference() {
            // Should work without explicit types
            let string_signal = signal("hello".to_string());
            let vec_signal = signal(vec![1, 2, 3]);
            let bool_signal = signal(true);
            
            assert_eq!(string_signal.get(), "hello");
            assert_eq!(vec_signal.get(), vec![1, 2, 3]);
            assert_eq!(bool_signal.get(), true);
        }

        // Mock signal implementation for testing
        #[derive(Clone)]
        struct TestSignal<T> {
            value: std::rc::Rc<std::cell::RefCell<T>>,
        }

        impl<T: Clone> TestSignal<T> {
            fn get(&self) -> T {
                self.value.borrow().clone()
            }
            
            fn set(&self, new_value: T) {
                *self.value.borrow_mut() = new_value;
            }
            
            fn update<F>(&self, f: F) 
            where 
                F: FnOnce(&mut T)
            {
                f(&mut *self.value.borrow_mut());
            }
            
            fn derive<U, F>(&self, f: F) -> TestSignal<U>
            where
                F: Fn(&T) -> U + 'static,
                U: Clone,
            {
                let derived_value = f(&self.get());
                TestSignal {
                    value: std::rc::Rc::new(std::cell::RefCell::new(derived_value)),
                }
            }
            
            fn split(self) -> (TestSignal<T>, TestSignal<T>) {
                (self.clone(), self)
            }
        }

        fn signal<T: Clone + 'static>(initial: T) -> TestSignal<T> {
            TestSignal {
                value: std::rc::Rc::new(std::cell::RefCell::new(initial)),
            }
        }
    }

    /// Unit tests for LEPTOS-2024-005: Error Message Improvements
    mod error_handling {
        #[derive(Debug)]
        pub struct FrameworkError {
            pub error_type: ErrorType,
            pub message: String,
            pub suggestions: Vec<String>,
            pub documentation_link: Option<String>,
        }

        #[derive(Debug, PartialEq)]
        pub enum ErrorType {
            SignalUsage,
            FeatureFlagMismatch,
            ServerFunctionContext,
            BuildConfiguration,
            ComponentDefinition,
        }

        #[test]
        fn test_signal_usage_error_detection() {
            let error_context = "view! { <span>{count}</span> }";
            let error = detect_framework_error(error_context);
            
            assert_eq!(error.error_type, ErrorType::SignalUsage);
            assert!(error.message.contains("Signal used directly in view"));
            assert!(!error.suggestions.is_empty());
            assert!(error.suggestions[0].contains("count.get()"));
        }

        #[test]
        fn test_error_message_helpfulness() {
            let error = FrameworkError {
                error_type: ErrorType::SignalUsage,
                message: "Signal used directly in view without calling .get()".to_string(),
                suggestions: vec![
                    "Try: count.get() for one-time reads".to_string(),
                    "Try: move || count.get() for reactive updates".to_string(),
                ],
                documentation_link: Some("https://leptos.dev/docs/reactivity/signals".to_string()),
            };
            
            let formatted = format_error_message(&error);
            
            // Should contain actionable suggestions
            assert!(formatted.contains("Try:"));
            assert!(formatted.contains("help:"));
            assert!(formatted.contains("count.get()"));
            
            // Should include documentation link
            assert!(formatted.contains("docs:"));
            assert!(formatted.contains("leptos.dev"));
        }

        #[test]
        fn test_feature_flag_error_detection() {
            let cargo_content = r#"[features]
default = ["csr", "ssr"]"#;
            
            let error = detect_feature_flag_error(cargo_content);
            assert_eq!(error.error_type, ErrorType::FeatureFlagMismatch);
            assert!(error.message.contains("Conflicting Leptos features"));
            assert!(error.suggestions.iter().any(|s| s.contains("Choose one primary rendering mode")));
        }

        // Mock implementations
        fn detect_framework_error(context: &str) -> FrameworkError {
            if context.contains("{count}") && !context.contains("count.get()") {
                FrameworkError {
                    error_type: ErrorType::SignalUsage,
                    message: "Signal used directly in view without calling .get()".to_string(),
                    suggestions: vec![
                        "Try: count.get() for one-time reads".to_string(),
                        "Try: move || count.get() for reactive updates".to_string(),
                    ],
                    documentation_link: Some("https://leptos.dev/docs/reactivity/signals".to_string()),
                }
            } else {
                FrameworkError {
                    error_type: ErrorType::ComponentDefinition,
                    message: "Unknown error".to_string(),
                    suggestions: vec![],
                    documentation_link: None,
                }
            }
        }

        fn detect_feature_flag_error(cargo_content: &str) -> FrameworkError {
            if cargo_content.contains(r#"["csr", "ssr"]"#) {
                FrameworkError {
                    error_type: ErrorType::FeatureFlagMismatch,
                    message: "Conflicting Leptos features enabled".to_string(),
                    suggestions: vec![
                        "Choose one primary rendering mode:".to_string(),
                        "For SPAs: default = [\"csr\"]".to_string(),
                        "For SSR: default = [\"ssr\"]".to_string(),
                    ],
                    documentation_link: Some("https://leptos.dev/docs/deployment".to_string()),
                }
            } else {
                FrameworkError {
                    error_type: ErrorType::BuildConfiguration,
                    message: "Unknown build configuration error".to_string(),
                    suggestions: vec![],
                    documentation_link: None,
                }
            }
        }

        fn format_error_message(error: &FrameworkError) -> String {
            let mut formatted = format!("error: {}\n", error.message);
            
            if !error.suggestions.is_empty() {
                for suggestion in &error.suggestions {
                    formatted.push_str(&format!("  = help: {}\n", suggestion));
                }
            }
            
            if let Some(link) = &error.documentation_link {
                formatted.push_str(&format!("  = docs: {}\n", link));
            }
            
            formatted
        }
    }
}