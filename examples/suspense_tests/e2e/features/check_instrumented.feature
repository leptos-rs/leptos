@check_instrument
Feature: Check Instrumented

    Background:

        Given I see the app

    @check_instrumented-initial
    Scenario Outline: Should see fresh counters
        Given I select the mode Instrumented
        When I select the component Counters
        When I click on Reset Counters
        Then I see the Counters under the Suspend Calls
            | item_listing       | 0 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        Then I see the Counters under the Server Calls
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |
