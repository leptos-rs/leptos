@click_inside_component_count
Feature: Click Inside Component Count

    Background:

        Given I see the app

    Scenario Outline: Should increase the count

        Given I select the mode <Mode>
        And I select the component Inside Component
        When I click the count 3 times
        Then I see the count is 3

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |