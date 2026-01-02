@check_issue_4492
Feature: Regression test for issue #4492

	Scenario: Scenario A should show Loading once on first load.
		Given I see the app
		And I can access regression test 4492
		When I click the button a-toggle
		Then I see a-result has the text Loading...
		When I wait 100ms
		Then I see a-result has the text 0
		When I click the button a-button
		Then I see a-result has the text 0
		When I wait 100ms
		Then I see a-result has the text 1

	Scenario: Scenario B should never show Loading
		Given I see the app
		And I can access regression test 4492
		When I click the button b-toggle
		Then I see b-result has the text 0
		When I click the button b-button
		Then I see b-result has the text 0
		When I wait 100ms
		Then I see b-result has the text 1
		When I click the button b-button
		Then I see b-result has the text 1
		When I wait 100ms
		Then I see b-result has the text 2

	Scenario: Scenario C should never show Loading
		Given I see the app
		And I can access regression test 4492
		When I click the button c-toggle
		Then I see c-result has the text 0
		When I click the button c-button
		Then I see c-result has the text 42
		When I wait 100ms
		Then I see c-result has the text 1
