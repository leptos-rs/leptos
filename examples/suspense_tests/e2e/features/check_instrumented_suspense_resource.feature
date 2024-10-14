@check_instrumented_suspense_resource
Feature: Using instrumented counters for real
    Check that the suspend/suspense and the underlying resources are
    called with the expected number of times for CSR rendering.

    Background:

        Given I see the app
        And I select the mode Instrumented

    Scenario: Emulate steps 1 to 5 of issue #2961
        Given I select the link Target 3##
        And I refresh the page
        When I select the following links
            | Item Listing |
            | Target 4##   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 2 |
            | item_inspect       | 0 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 2 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    Scenario: Emulate step 6 of issue #2961
        Given I select the link Target 41#
        And I refresh the page
        When I select the following links
            | Target 4##   |
            | Target 42#   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 1 |
            | item_inspect       | 2 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 1 |
            | inspect_item_root  | 2 |
            | inspect_item_field | 0 |

    Scenario: Emulate step 7 of issue #2961
        Given I select the link Target 42#
        And I refresh the page
        When I select the following links
            | Target 4##   |
            | Target 42#   |
            | Target 41#   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 0 |
            | item_overview      | 1 |
            | item_inspect       | 3 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 1 |
            | inspect_item_root  | 3 |
            | inspect_item_field | 0 |

    Scenario: Emulate step 8, "not trigger double fetch".
        Given I select the link Target 3##
        And I refresh the page
        When I select the following links
            | Item Listing |
            | Target 4##   |
            | Target 41#   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 2 |
            | item_inspect       | 1 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 2 |
            | inspect_item_root  | 1 |
            | inspect_item_field | 0 |

    Scenario: Like above, for the "double fetch" which shouldn't happen
        Given I select the link Target 3##
        And I refresh the page
        When I select the following links
            | Item Listing |
            | Target 4##   |
            | Target 41#   |
            | Target 3##   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 3 |
            | item_inspect       | 1 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 3 |
            | inspect_item_root  | 1 |
            | inspect_item_field | 0 |

    Scenario: Like above, but using 4## instead
        Given I select the link Target 3##
        And I refresh the page
        When I select the following links
            | Item Listing |
            | Target 4##   |
            | Target 41#   |
            | Target 4##   |
        And I go check the Counters
        Then I see the following counters under section
            | Suspend Calls      |   |
            | item_listing       | 1 |
            | item_overview      | 3 |
            | item_inspect       | 1 |
        And the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 2 |
            | inspect_item_root  | 1 |
            | inspect_item_field | 0 |

    # Currently, get_item is invoked with `3` as the argument upon
    # selection of `Item Listing` despite that `Item Listing` doesn't
    # need `get_item` calls.  Seems like it may be due to the system
    # still reacting to the unmounting of the component that needed
    # view that generated the original `Item 3` (hydrated from SSR).
    # Tests above may also have this type of behavior, but is somewhat
    # masked because the direction of going down and then back up, but
    # if this behavior changes for the better (avoiding this spurious
    # resource fetch) then the above tests may need updating to reflect
    # the corrected behavior.  Note the difference with the fully CSR
    # scenario after this one
    Scenario: Emulate part of step 8 of issue #2961
        Given I select the link Target 3##
        And I refresh the page
        When I select the link Item Listing
        And I go check the Counters
        Then I see the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 1 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    # Instead of refreshing the page like above, CSR counters is reset
    # instead to keep the starting counter conditions identical.
    Scenario: Emulate above, instead of refresh page, reset csr counters
        Given I select the link Target 3##
        And I click on Reset CSR Counters
        When I select the link Item Listing
        And I go check the Counters
        Then I see the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |

    # Again, the following two sets demostrates resources making stale
    # and redundant requests when hydrated, and not do so when under
    # CSR.
    Scenario: Start with hydration from Target 41# and go up
        Given I select the link Target 41#
        And I refresh the page
        When I select the link Target 4##
        And I select the link Item Listing
        And I go check the Counters
        Then I see the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 1 |
            | inspect_item_root  | 1 |
            | inspect_item_field | 0 |

    Scenario: Start with hydration from Target 41# and go up
        Given I select the link Target 41#
        And I click on Reset CSR Counters
        When I select the link Target 4##
        And I select the link Item Listing
        And I go check the Counters
        Then I see the following counters under section
            | Server Calls (CSR) |   |
            | list_items         | 0 |
            | get_item           | 0 |
            | inspect_item_root  | 0 |
            | inspect_item_field | 0 |
