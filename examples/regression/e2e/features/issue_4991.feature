@check_issue_4991
Feature: Check that issue 4991 does not reappear

	Scenario: Nested route params are decoded once when rendered on the server
		Given I load the path /4991/x%2541/tags/a%252Bb
		Then I see section has the text Some("x%41")
		And I see tag has the text Some("a%2Bb")

	Scenario: Nested route params are decoded once after client-side navigation
		Given I see the app
		And I can access regression test 4991
		Then I see section has the text Some("x%41")
		And I see tag has the text Some("a%2Bb")
		When I select the link other tag
		Then I see section has the text Some("x%41")
		And I see tag has the text Some("c%26d")
