@check_issue_4881
Feature: Check that issue 4881 does not reappear

	Scenario: Hydrating a <For> whose items each create a Resource
		Given I see the app
		And I can access regression test 4881
		When I refresh the browser
		And I wait 500ms
		Then I see files-1 has the text item 1 file 0
		And I see files-2 has the text item 2 file 0, item 2 file 1
		And I see files-3 has the text item 3 file 0, item 3 file 1, item 3 file 2
		When I click the button bump-1
		And I click the button bump-2
		And I click the button bump-3
		Then I see count-1 has the text 1
		And I see count-2 has the text 1
		And I see count-3 has the text 1
