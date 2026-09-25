@chunk_failure
Feature: A failed lazy-route chunk load renders the ErrorBoundary fallback

	Scenario: A chunk-fetch failure shows the fallback instead of crashing the app.
		Given I see the app
		Then I see the page is View A
		When wasm chunk requests fail
		When I select the link C
		Then I see the chunk error fallback

	Scenario: A transient chunk-fetch failure is not cached, so navigating again recovers.
		Given I see the app
		Then I see the page is View A
		When wasm chunk requests fail
		When I select the link C
		Then I see the chunk error fallback
		When wasm chunk requests succeed again
		When I select the link A
		Then I see the page is View A
		When I select the link C
		When I wait for a second
		Then I see the page is View C
