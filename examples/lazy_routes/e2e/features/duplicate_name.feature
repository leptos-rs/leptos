@duplicate_names
Feature: Lazy functions can share the same name

	Scenario: Two functions with the same name both work.
		Given I see the app
		Then I see the page is View A
		When I click the button First
		When I wait for a second
		Then I see the result is {"a":"First Value","b":1}
		When I click the button Second
		When I wait for a second
		Then I see the result is {"a":"Second Value","b":2}
		When I click the button Third
		When I wait for a second
		Then I see the result is Third value.