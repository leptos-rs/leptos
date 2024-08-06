# E2E Testing

This example demonstrates e2e testing with Rust using executable requirements.

## Testing Stack

|    |      Role      |  Description |
|---|---|---|
| [Cucumber](https://github.com/cucumber-rs/cucumber/tree/main) | Test Runner | Run [Gherkin](https://cucumber.io/docs/gherkin/reference/) specifications as Rust tests |
| [Fantoccini](https://github.com/jonhoo/fantoccini/tree/main) | Browser Client | Interact with web pages through WebDriver |
| [Cargo Leptos](https://github.com/leptos-rs/cargo-leptos) | Build Tool |  Compile example and start the server and end-2-end tests |
| [chromedriver](https://chromedriver.chromium.org/downloads) | WebDriver | Provide WebDriver for Chrome |

## Testing Organization

Testing is organized around what a user can do and see/not see. Test scenarios are grouped by the **user action** and the **object** of that action. This makes it easier to locate and reason about requirements.

Here is a brief overview of how things fit together.

```bash
features
└── {action}_{object}.feature   # Specify test scenarios
tests
├── fixtures
│   ├── action.rs               # Perform a user action (click, type, etc.)
│   ├── check.rs                # Assert what a user can see/not see
│   ├── find.rs                 # Query page elements
│   ├── mod.rs
│   └── world
│       ├── action_steps.rs     # Map Gherkin steps to user actions
│       ├── check_steps.rs      # Map Gherkin steps to user expectations
│       └── mod.rs
└── app_suite.rs                # Test main 
```
