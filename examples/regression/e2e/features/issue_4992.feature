@check_issue_4992
Feature: Check that issue 4992 does not reappear

	Scenario: query_signal setter writes the value to the URL without percent-decoding it
		Given I see the app
		And I can access regression test 4992
		Then I see value has the text None
		When I click the button set-q
		And I wait 200ms
		Then I see search has the text q=a%252Bb
		And I see value has the text Some("a%2Bb")
		When I reload the page
		Then I see search has the text q=a%252Bb
		And I see value has the text Some("a%2Bb")
