@check_instrumented_issue_3719
Feature: Using instrumented counters to test regression from #3502.
    Check that the suspend/suspense and the underlying resources are
    called with the expected number of times.  If this was already in
    place by #3502 (5c43c18) it should have caught this regression.
    For a better minimum demonstration see #3719.

    Background:

        Given I see the app
        And I select the mode Instrumented

    Scenario: follow all paths via CSR avoids #3502
        Given I select the following links
            | Item Listing         |
            | Item 1               |
            | Inspect path2        |
            | Inspect path2/field3 |
        And I click on Reset CSR Counters
        When I select the following links
            | Inspect path2/field1 |
            | Inspect path2/field2 |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 0 |
            | item_inspect       | 2 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 2 |

    # To show that starting directly from within a param will simply
    # cause the problem.
    Scenario: Quicker way to demonstrate regression caused by #3502
        Given I select the link Target 123
        # And I click on Reset CSR Counters
        When I select the following links
            | Inspect path2/field1 |
            | Inspect path2/field2 |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 0 |
            | item_inspect       | 3 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 1 |
            | get_item           | 1 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 4 |

    Scenario: Follow paths ordinarily down to a target
        Given I select the following links
            | Item Listing         |
            | Item 1               |
        And I click on Reset CSR Counters
        When I select the following links
            | Target 4## |
            | Target 3## |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 2 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 2 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    Scenario: Same as above, but add a refresh to test hydration
        Given I select the following links
            | Item Listing         |
            | Item 1               |
        And I refresh the page
        And I click on Reset CSR Counters
        When I select the following links
            | Target 4## |
            | Target 3## |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 2 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 2 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

