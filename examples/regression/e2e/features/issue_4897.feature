@check_issue_4897
Feature: Check that issue 4897 does not reappear

	Scenario: Hydrating sibling <Suspense/>s whose nested resources resolve out of order
		Given I see the app
		And I can access regression test 4897
		When I refresh the browser
		And I wait 500ms
		Then I see slow-value has the text 5
		And I see fast-value has the text 9
		When I click the button slow-bump
		And I click the button fast-bump
		Then I see slow-value has the text 6
		And I see fast-value has the text 10

	Scenario: Hydrating an error thrown inside a nested <Suspense/>
		Given I see the app
		And I can access regression test 4897
		When I refresh the browser
		And I wait 500ms
		Then I see error-count has the text 1
		When I click the button error-bump
		Then I see error-clicks has the text 1
