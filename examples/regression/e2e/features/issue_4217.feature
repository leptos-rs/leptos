@check_issue_4217
Feature: Check that issue 4217 does not reappear

	Scenario: All items are selected.
		Given I see the app
		And I can access regression test 4217
		Then I see option1 is selected
		And I see option2 is selected
		And I see option3 is selected