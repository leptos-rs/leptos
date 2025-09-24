@check_issue_4324
Feature: Check that issue 4324 does not reappear

	Scenario: Navigating to the same page after clicking "Back" should set the URL correctly
		Given I see the app
		And I can access regression test 4324
		Then I see the path is /4324/
		When I press the back button
		Then I see the path is /
		When I select the link 4324
		Then I see the path is /4324/