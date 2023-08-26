@click_parallel_counts_1
Feature: Click Parallel Count (1)

    Background:

        Given I see the app

    Scenario Outline: Should increase the first and second counts

        Given I select the mode <Mode>
        And I select the component Parallel
        When I click the first count 3 times
        Then I see the first count is 3
        And I see the second count is 3

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |
