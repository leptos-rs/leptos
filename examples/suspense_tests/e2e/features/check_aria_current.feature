@check_aria_current
Feature: Check aria-current being applied to make links bolded

    Background:

        Given I see the app

    Scenario: Should see the base case working
        Then I see the link Out-of-Order being bolded
        Then I see the following links being bolded
            | Out-of-Order |
            | Nested       |
