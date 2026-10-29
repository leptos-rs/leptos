@check_issue_4968
Feature: Check that issue 4968 does not reappear

	Scenario: Unmounting a rebuilt <Title/> falls back to the outer title
		Given I see the app
		And I can access regression test 4968
		Then I see the page title is Inner 0
		When I click the button rename
		Then I see the page title is Inner 1
		When I click the button toggle
		Then I see the page title is Outer

	Scenario: Unmounting a rebuilt reactive <Title/> does not read a disposed signal
		Given I see the app
		And I can access regression test 4968
		When I click the button toggle
		Then I see the page title is Outer
		When I click the button toggle-reactive
		Then I see the page title is Reactive 0
		When I click the button rename-reactive
		Then I see the page title is Reactive 1
		When I click the button toggle-reactive
		Then I see the page title is Outer
		When I click the button toggle
		Then I see the page title is Inner 0
