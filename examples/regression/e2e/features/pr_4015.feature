@check_pr_4015
Feature: Check that PR 4015 does not regress

	Scenario: The correct text appears
		Given I see the app
		And I can access regression test 4015
		Then I see the result is the string Some(42)

