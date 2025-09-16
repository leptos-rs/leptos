@check_issue_4285
Feature: Check that issue 4285 does not reappear

	Scenario: Navigating several times to same lazy route does not cause issues.
		Given I see the app
		And I can access regression test 4285
		And I can access regression test 4285
		And I can access regression test 4285
		Then I see the result is the string 42
