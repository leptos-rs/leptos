@check_issue_5000
Feature: Check that issue 5000 does not reappear

	Scenario: Clicking a link keeps a double-encoded path in the address bar
		Given I see the app
		And I can access regression test 5000
		When I select the link x%2541
		And I wait 200ms
		Then I see the path is /5000/x%2541
		And I see pathname has the text /5000/x%2541
		When I reload the page
		Then I see the path is /5000/x%2541
		And I see pathname has the text /5000/x%2541

	Scenario: Clicking a link keeps an encoded backslash in the address bar
		Given I see the app
		And I can access regression test 5000
		When I select the link a%5Cb
		And I wait 200ms
		Then I see the path is /5000/a%5Cb
		And I see pathname has the text /5000/a%5Cb
		When I reload the page
		Then I see the path is /5000/a%5Cb
		And I see pathname has the text /5000/a%5Cb

	Scenario: Going back to a clicked link resolves the same path
		Given I see the app
		And I can access regression test 5000
		When I select the link x%2541
		And I wait 200ms
		And I select the link back to links
		And I wait 200ms
		And I press the back button
		And I wait 200ms
		Then I see the path is /5000/x%2541
		And I see pathname has the text /5000/x%2541
