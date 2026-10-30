@check_issue_4980
Feature: Check that issue 4980 does not reappear

	Scenario: <ErrorBoundary/> around <Await/> does not swap content with a sibling <Suspense/>
		Given I see the app
		And I can access regression test 4980
		Then I see await-items has the text 1,2,3
		When I refresh the browser
		And I wait 500ms
		Then I see await-items has the text 1,2,3
		And I see first has the text 1,2,3,4
		And I see second has the text 1,2,3,4,5
		When I click the button await-bump
		Then I see await-count has the text 1
