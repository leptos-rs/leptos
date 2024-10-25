@check_instrumented
Feature: Instrumented Counters showing the expected values

    Scenario: I can fresh CSR instrumented counters
        Given I see the app
        When I access the instrumented counters via CSR
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    Scenario: I should see counter going up after viewing Item Listing
        Given I see the app
        When I select the following links
            | Instrumented |
            | Item Listing |
            | Counters     |
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 1 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    # the reload has happened in Item Listing, it follows a suspend
    # will be called as hydration happens.
    Scenario: Refreshing Item Listing should have only suspend counters
        Given I see the app
        When I access the instrumented counters via SSR
        And I select the component Item Listing
        And I reload the page
        And I select the component Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    Scenario: Reset CSR Counters work as expected.
        Given I see the app
        When I access the instrumented counters via SSR
        And I select the component Item Listing
        And I click on Reset CSR Counters
        And I select the component Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 0 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    Scenario: Standard usage of the instruments traversing down
        Given I see the app
        When I select the following links
            | Instrumented         |
            | Item Listing         |
            | Item 2               |
            | Inspect path3        |
            | Inspect path3/field1 |
        And I access the instrumented counters via CSR
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 1 |
            | item_inspect       | 2 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 1 |
            | get_item           | 1 |
            | inspect_item_root  | 1 |
            | inspect_item_field | 1 |
