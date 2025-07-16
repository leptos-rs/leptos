@shared_chunks
Feature: Shared code splitting works correctly

	Scenario: Two functions using same serde code both work.
		Given I see the app
		Then I see the page is View A
		When I click the button First
		When I wait for a second
		Then I see the result is {"a":"First Value","b":1}