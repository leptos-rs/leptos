@check_instrument_item_listing
Feature: Check Instrumented Item Listing

    Background:

        Given I see the app

    @check_instrumented-initial
    Scenario Outline: Should see counters updated
        Given I select the mode Instrumented
        When I select the component Counters
        When I click on Reset Counters
        When I select the component Item Listing
        When I select the component Counters
        Then I see the Counters under the Suspend Calls
            | item_listing       | 1 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        Then I see the Counters under the Server Calls
            | list_items         | 1 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |
