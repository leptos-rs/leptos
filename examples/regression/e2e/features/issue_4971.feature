@check_issue_4971
Feature: Check that issue 4971 does not reappear

	Scenario: Superseding a pending navigation keeps the displayed route's data alive
		Given I see the app
		And I can access regression test 4971
		When I click the button increment-4971
		Then I see the result is the string 1
		When I select the link Slow 4971
		And I select the link Other 4971
		And I click the button increment-4971
		Then I see the result is the string 2
		When I wait 2000ms
		Then I see page-4971 has the text Other
