@check_issue_4005
Feature: Check that issue 4005 does not reappear

	Scenario: The second item is selected.
		Given I see the app
		And I can access regression test 4005
		Then I see the value of select is 2