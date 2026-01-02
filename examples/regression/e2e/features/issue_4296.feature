@check_issue_4296
Feature: Check that issue 4296 does not reappear

	Scenario: Query param signals created in LazyRoute::data() are reactive in ::view().
		Given I see the app
		And I can access regression test 4296
		Then I see the result is the string None
		When I select the link abc
		Then I see the result is the string Some("abc")
		When I select the link def
		Then I see the result is the string Some("def")

	Scenario: Loading page with query signal works as well.
		Given I see the app
		And I can access regression test 4296
		When I select the link abc
		When I reload the page
		Then I see the result is the string Some("abc")
