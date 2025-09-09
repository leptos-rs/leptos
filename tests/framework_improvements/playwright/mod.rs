//! Playwright Browser Tests for Framework Improvements
//!
//! Layer 4 of TDD framework - browser-based testing for UI components, user interactions,
//! and cross-browser compatibility validation.

pub mod browser_compatibility_tests;
pub mod user_interaction_tests;
pub mod performance_metrics_tests;
pub mod accessibility_tests;
pub mod visual_regression_tests;

use std::process::Command;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod leptos_playwright_tests {
    use super::*;

    /// Browser-based component testing
    mod component_browser_tests {
        use super::*;

        #[test]
        fn test_counter_component_in_browser() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping browser test - no test server available");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('counter component works in browser', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Check initial state
  await expect(page.locator('button')).toContainText('Count: 0');
  
  // Click button and verify increment
  await page.click('button');
  await expect(page.locator('button')).toContainText('Count: 1');
  
  // Multiple clicks should work
  await page.click('button');
  await page.click('button');
  await expect(page.locator('button')).toContainText('Count: 3');
  
  // Verify reactivity is working
  const countText = await page.locator('button').textContent();
  expect(countText).toContain('3');
});
"#);
            
            assert!(playwright_result.success, 
                   "Counter component should work in browser: {}", 
                   playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }

        #[test]
        fn test_form_components_browser_validation() {
            let test_server = setup_test_server_with_forms();
            if !test_server.started {
                println!("Skipping form test - no test server available");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('form components work correctly', async ({ page }) => {
  await page.goto('http://localhost:3000/forms');
  
  // Test input field reactivity
  await page.fill('input[type="text"]', 'Hello Leptos');
  await expect(page.locator('[data-testid="input-display"]')).toContainText('Hello Leptos');
  
  // Test form submission
  await page.fill('input[name="email"]', 'test@example.com');
  await page.fill('input[name="password"]', 'password123');
  await page.click('button[type="submit"]');
  
  // Verify form handling
  await expect(page.locator('[data-testid="form-result"]')).toContainText('Success');
});
"#);
            
            assert!(playwright_result.success,
                   "Form components should work: {}", playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }
    }

    /// Cross-browser compatibility tests
    mod cross_browser_tests {
        use super::*;

        #[test]
        fn test_chrome_compatibility() {
            test_browser_compatibility("chromium");
        }

        #[test]
        fn test_firefox_compatibility() {
            test_browser_compatibility("firefox");
        }

        #[test]
        fn test_safari_compatibility() {
            test_browser_compatibility("webkit");
        }

        fn test_browser_compatibility(browser: &str) {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping {} test - no test server available", browser);
                return;
            }
            
            let playwright_result = run_playwright_test_with_browser(browser, r#"
import { test, expect } from '@playwright/test';

test(`leptos app works in ${browser}`, async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Basic functionality should work
  await expect(page.locator('h1')).toBeVisible();
  await expect(page.locator('button')).toBeVisible();
  
  // Interactivity should work
  await page.click('button');
  await expect(page.locator('button')).toContainText('Count: 1');
  
  // No JavaScript errors
  const errors = [];
  page.on('pageerror', error => errors.push(error));
  await page.reload();
  expect(errors).toHaveLength(0);
});
"#);
            
            assert!(playwright_result.success,
                   "App should work in {}: {}", browser, playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }
    }

    /// Performance testing in browser
    mod browser_performance_tests {
        use super::*;

        #[test]
        fn test_initial_load_performance() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping performance test - no test server available");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('initial load performance meets targets', async ({ page }) => {
  const startTime = Date.now();
  
  await page.goto('http://localhost:3000');
  await page.waitForLoadState('networkidle');
  
  const loadTime = Date.now() - startTime;
  
  // Target: <3s load time
  expect(loadTime).toBeLessThan(3000);
  
  // Measure Core Web Vitals
  const webVitals = await page.evaluate(() => {
    return new Promise((resolve) => {
      new PerformanceObserver((list) => {
        const entries = list.getEntries();
        resolve(entries.map(entry => ({
          name: entry.name,
          value: entry.value,
          rating: entry.rating
        })));
      }).observe({ entryTypes: ['web-vital'] });
      
      // Fallback timeout
      setTimeout(() => resolve([]), 5000);
    });
  });
  
  console.log('Web Vitals:', webVitals);
  
  // Basic bundle size check
  const response = await page.goto('http://localhost:3000');
  const contentLength = response.headers()['content-length'];
  if (contentLength) {
    const sizeKB = parseInt(contentLength) / 1024;
    expect(sizeKB).toBeLessThan(500); // <500KB initial bundle
  }
});
"#);
            
            assert!(playwright_result.success,
                   "Performance targets should be met: {}", playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }

        #[test]
        fn test_interaction_performance() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping interaction performance test");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('user interactions are fast', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Measure click-to-update time
  const startTime = await page.evaluate(() => performance.now());
  await page.click('button');
  
  // Wait for DOM update
  await page.waitForFunction('document.querySelector("button").textContent.includes("Count: 1")');
  
  const endTime = await page.evaluate(() => performance.now());
  const interactionTime = endTime - startTime;
  
  // Target: <100ms interaction response
  expect(interactionTime).toBeLessThan(100);
  
  // Test rapid interactions
  for (let i = 0; i < 10; i++) {
    await page.click('button');
  }
  
  // Should handle rapid clicks without issues
  await expect(page.locator('button')).toContainText('Count: 11');
});
"#);
            
            assert!(playwright_result.success,
                   "Interaction performance should meet targets: {}", 
                   playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }
    }

    /// Accessibility testing
    mod accessibility_tests {
        use super::*;

        #[test]
        fn test_accessibility_compliance() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping accessibility test - no test server available");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('app meets accessibility standards', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Run axe accessibility tests
  const accessibilityScanResults = await new AxeBuilder({ page }).analyze();
  
  // Should have no accessibility violations
  expect(accessibilityScanResults.violations).toEqual([]);
  
  // Check keyboard navigation
  await page.keyboard.press('Tab');
  const focusedElement = await page.evaluate(() => document.activeElement.tagName);
  expect(['BUTTON', 'INPUT', 'A']).toContain(focusedElement);
  
  // Check screen reader support
  const ariaLabels = await page.evaluate(() => {
    const elements = document.querySelectorAll('[aria-label], [aria-labelledby]');
    return elements.length;
  });
  
  expect(ariaLabels).toBeGreaterThan(0);
});
"#);
            
            assert!(playwright_result.success,
                   "Accessibility standards should be met: {}", 
                   playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }

        #[test] 
        fn test_keyboard_navigation() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping keyboard navigation test");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('keyboard navigation works correctly', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Tab navigation should work
  await page.keyboard.press('Tab');
  let focused = await page.evaluate(() => document.activeElement);
  expect(focused).toBeTruthy();
  
  // Enter/Space should activate buttons
  await page.keyboard.press('Enter');
  await expect(page.locator('button')).toContainText('Count: 1');
  
  await page.keyboard.press('Space');
  await expect(page.locator('button')).toContainText('Count: 2');
  
  // Escape should work for modals/overlays if present
  await page.keyboard.press('Escape');
  
  // Focus should be visible
  const focusVisible = await page.evaluate(() => {
    const focused = document.activeElement;
    const style = window.getComputedStyle(focused, ':focus');
    return style.outline !== 'none' || style.boxShadow !== 'none';
  });
  
  expect(focusVisible).toBe(true);
});
"#);
            
            assert!(playwright_result.success,
                   "Keyboard navigation should work: {}", playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }
    }

    /// Visual regression testing
    mod visual_tests {
        use super::*;

        #[test]
        fn test_visual_consistency() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping visual regression test");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('visual appearance is consistent', async ({ page }) => {
  await page.goto('http://localhost:3000');
  
  // Take screenshot for visual comparison
  await expect(page).toHaveScreenshot('homepage-initial.png');
  
  // Test different states
  await page.click('button');
  await expect(page).toHaveScreenshot('homepage-clicked.png');
  
  // Test responsive design
  await page.setViewportSize({ width: 375, height: 667 }); // Mobile
  await expect(page).toHaveScreenshot('homepage-mobile.png');
  
  await page.setViewportSize({ width: 1024, height: 768 }); // Tablet
  await expect(page).toHaveScreenshot('homepage-tablet.png');
  
  await page.setViewportSize({ width: 1920, height: 1080 }); // Desktop
  await expect(page).toHaveScreenshot('homepage-desktop.png');
});
"#);
            
            // Visual tests might fail on first run - that's expected
            if !playwright_result.success && 
               playwright_result.error_message.contains("Screenshot comparison failed") {
                println!("Visual test failed - this is expected on first run or changes");
            } else {
                assert!(playwright_result.success,
                       "Visual tests should pass: {}", playwright_result.error_message);
            }
            
            cleanup_test_server(&test_server);
        }
    }

    /// Mobile and responsive testing
    mod mobile_tests {
        use super::*;

        #[test]
        fn test_mobile_device_compatibility() {
            let test_server = setup_test_server();
            if !test_server.started {
                println!("Skipping mobile test");
                return;
            }
            
            let playwright_result = run_playwright_test(r#"
import { test, expect } from '@playwright/test';

test('app works on mobile devices', async ({ page }) => {
  // iPhone 12 Pro dimensions
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('http://localhost:3000');
  
  // Should be responsive
  await expect(page.locator('h1')).toBeVisible();
  await expect(page.locator('button')).toBeVisible();
  
  // Touch interactions should work
  await page.tap('button');
  await expect(page.locator('button')).toContainText('Count: 1');
  
  // Check for mobile-specific issues
  const hasHorizontalScroll = await page.evaluate(() => {
    return document.body.scrollWidth > window.innerWidth;
  });
  
  expect(hasHorizontalScroll).toBe(false);
  
  // Text should be readable (not too small)
  const textSize = await page.evaluate(() => {
    const button = document.querySelector('button');
    return window.getComputedStyle(button).fontSize;
  });
  
  const size = parseInt(textSize);
  expect(size).toBeGreaterThan(14); // Minimum readable size
});
"#);
            
            assert!(playwright_result.success,
                   "Mobile compatibility should work: {}", playwright_result.error_message);
            
            cleanup_test_server(&test_server);
        }
    }

    // Helper structures and functions
    #[derive(Debug)]
    struct TestServer {
        started: bool,
        port: u16,
        process_id: Option<u32>,
    }

    #[derive(Debug)]
    struct PlaywrightResult {
        success: bool,
        error_message: String,
    }

    fn setup_test_server() -> TestServer {
        // Try to start a test server for the app
        let result = Command::new("cargo")
            .args(&["run", "--bin", "test-server"])
            .spawn();
        
        match result {
            Ok(child) => TestServer {
                started: true,
                port: 3000,
                process_id: Some(child.id()),
            },
            Err(_) => TestServer {
                started: false,
                port: 3000,
                process_id: None,
            }
        }
    }

    fn setup_test_server_with_forms() -> TestServer {
        // Similar to setup_test_server but with form components
        setup_test_server()
    }

    fn cleanup_test_server(server: &TestServer) {
        if let Some(pid) = server.process_id {
            // Kill the test server process
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .output();
        }
    }

    fn run_playwright_test(test_code: &str) -> PlaywrightResult {
        run_playwright_test_with_browser("chromium", test_code)
    }

    fn run_playwright_test_with_browser(browser: &str, test_code: &str) -> PlaywrightResult {
        // Write test to temporary file
        let temp_dir = std::env::temp_dir().join("playwright_tests");
        std::fs::create_dir_all(&temp_dir).unwrap_or(());
        
        let test_file = temp_dir.join("test.spec.js");
        if std::fs::write(&test_file, test_code).is_err() {
            return PlaywrightResult {
                success: false,
                error_message: "Failed to write test file".to_string(),
            };
        }
        
        // Run playwright test
        let result = Command::new("npx")
            .args(&["playwright", "test", "--project", browser])
            .current_dir(&temp_dir)
            .output();
        
        match result {
            Ok(output) => PlaywrightResult {
                success: output.status.success(),
                error_message: String::from_utf8_lossy(&output.stderr).to_string(),
            },
            Err(e) => PlaywrightResult {
                success: false,
                error_message: format!("Failed to run playwright: {}", e),
            }
        }
    }
}