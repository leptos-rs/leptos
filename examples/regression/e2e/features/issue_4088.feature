@check_issue_4088
Feature: Check that issue 4088 does not reappear

	Scenario: I can see the navbar
		Given I see the app
		And I can access regression test 4088
		Then I see the navbar

	Scenario: The user info is shared via context
		Given I see the app
		And I can access regression test 4088
		When I select the link Class 1
		Then I see the result is the string Assignments for team of user with id 42

	Scenario: The user info is shared via context
		Given I see the app
		And I can access regression test 4088
		When I select the link Class 1
		When I refresh the browser
		Then I see the result is the string Assignments for team of user with id 42
