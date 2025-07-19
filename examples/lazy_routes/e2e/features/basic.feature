@basic
Feature: Check that each page hydrates correctly

	Scenario: Page A is rendered correctly.
		Given I see the app
		Then I see the page is View A

	Scenario: Page A hydrates and allows navigating to page B.
		Given I see the app
		When I select the link B
		Then I see the navigating indicator
		When I wait for a second
		Then I see the page is View B

	Scenario: Page B is rendered correctly.
		When I open the app at /b
		Then I see the page is View B

	Scenario: Page B hydrates and allows navigating to page C.
		When I open the app at /b
		When I select the link C
		Then I see the navigating indicator
		When I wait for a second
		Then I see the page is View C

	Scenario: Page C is rendered correctly.
		When I open the app at /c
		Then I see the page is View C

	Scenario: Page C hydrates and allows navigating to page A.
		When I open the app at /c
		When I select the link A
		Then I see the page is View A