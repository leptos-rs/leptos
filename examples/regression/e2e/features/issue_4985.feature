@check_issue_4985
Feature: Check that issue 4985 does not reappear

	Scenario: Lazy route data is serialized once when its outlet is inside <Suspense/>
		Given I load the path /4985/suspense
		Then I see result has the text 17
		And I see the page source contains ISSUE_4985_MARKER 1 time

	Scenario: Lazy route data is serialized once when its outlet is not inside <Suspense/>
		Given I load the path /4985/plain
		Then I see result has the text 17
		And I see the page source contains ISSUE_4985_MARKER 1 time
