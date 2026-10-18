@check_issue_4908
Feature: Check that issue 4908 does not reappear

	Scenario: <ErrorBoundary/> around <Suspense/> renders and hydrates its children (out-of-order)
		Given I see the app
		And I can access regression test 4908
		When I refresh the browser
		And I wait 500ms
		Then I see eb-suspense-ok-items has the text 1,2,3
		When I click the button eb-suspense-ok-bump
		Then I see eb-suspense-ok-count has the text 1

	Scenario: <ErrorBoundary/> around <Suspense/> renders and hydrates its fallback (out-of-order)
		Given I see the app
		And I can access regression test 4908
		When I refresh the browser
		And I wait 500ms
		Then I see eb-suspense-err-error-count has the text 1
		When I click the button eb-suspense-err-error-bump
		Then I see eb-suspense-err-error-clicks has the text 1

	Scenario: <Suspense/> around <ErrorBoundary/> renders and hydrates its children (out-of-order)
		Given I see the app
		And I can access regression test 4908
		When I refresh the browser
		And I wait 500ms
		Then I see suspense-eb-ok-items has the text 1,2,3
		When I click the button suspense-eb-ok-bump
		Then I see suspense-eb-ok-count has the text 1

	Scenario: <Suspense/> around <ErrorBoundary/> renders and hydrates its fallback (out-of-order)
		Given I see the app
		And I can access regression test 4908
		When I refresh the browser
		And I wait 500ms
		Then I see suspense-eb-err-error-count has the text 1
		When I click the button suspense-eb-err-error-bump
		Then I see suspense-eb-err-error-clicks has the text 1

	Scenario: <ErrorBoundary/> around <Suspense/> renders and hydrates its children (in-order)
		Given I see the app
		And I can access regression test 4908-in-order
		When I refresh the browser
		And I wait 500ms
		Then I see eb-suspense-ok-items has the text 1,2,3
		When I click the button eb-suspense-ok-bump
		Then I see eb-suspense-ok-count has the text 1

	Scenario: <ErrorBoundary/> around <Suspense/> renders and hydrates its fallback (in-order)
		Given I see the app
		And I can access regression test 4908-in-order
		When I refresh the browser
		And I wait 500ms
		Then I see eb-suspense-err-error-count has the text 1
		When I click the button eb-suspense-err-error-bump
		Then I see eb-suspense-err-error-clicks has the text 1

	Scenario: <Suspense/> around <ErrorBoundary/> renders and hydrates its children (in-order)
		Given I see the app
		And I can access regression test 4908-in-order
		When I refresh the browser
		And I wait 500ms
		Then I see suspense-eb-ok-items has the text 1,2,3
		When I click the button suspense-eb-ok-bump
		Then I see suspense-eb-ok-count has the text 1

	Scenario: <Suspense/> around <ErrorBoundary/> renders and hydrates its fallback (in-order)
		Given I see the app
		And I can access regression test 4908-in-order
		When I refresh the browser
		And I wait 500ms
		Then I see suspense-eb-err-error-count has the text 1
		When I click the button suspense-eb-err-error-bump
		Then I see suspense-eb-err-error-clicks has the text 1
