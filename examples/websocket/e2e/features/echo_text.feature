@echo_text
Feature: Echo Text

    Background:
        Given I see the app

    @echo_text-see-first-input
    Scenario: Should see the label
        Given I add a text as a
        Then I see the label of the input is A

    @add_text-see-second-input
    Scenario: Should see the label
        Given I add a text as ab
        Then I see the label of the input is AB


