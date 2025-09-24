@check_issue_4251
Feature: Check that issue 4251 does not reappear

	Scenario: Clicking a link to the same page you’re currently on should not add the page to the history stack.
		Given I see the app
		And I can access regression test 4324
		When I select the link This page
		And I select the link This page
		And I select the link This page
		Then I see the result is the string Issue4324
		When I press the back button
		And I select the link 4324
		Then I see the result is the string Issue4324