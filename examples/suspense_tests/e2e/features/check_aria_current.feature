@check_aria_current
Feature: Check aria-current being applied to make links bolded

    Background:

        Given I see the app

    Scenario: Should see the base case working
        Then I see the Out-of-Order link being bolded
        And I see the following links being bolded
            | Out-of-Order |
            | Nested       |
        And I see the In-Order link not being bolded
        And I see the following links not being bolded
            | In-Order     |
            | Single       |

    Scenario: Should see client-side render the correct bolded links
        When I select the link In-Order
        And I select the link Single
        Then I see the following links being bolded
            | In-Order     |
            | Single       |
        And I see the following links not being bolded
            | Out-of-Order |
            | Nested       |

    Scenario: Should see server-side render the correct bolded links
        When I select the link In-Order
        And I select the link Single
        And I reload the page
        Then I see the following links being bolded
            | In-Order     |
            | Single       |
        And I see the following links not being bolded
            | Out-of-Order |
            | Nested       |

    Scenario: Check that the base nested route links are working
        When I select the link Instrumented
        Then I see the Instrumented link being bolded
        And I see the Item Listing link not being bolded

    Scenario: Should see going deep down into nested routes bold links
        When I select the link Instrumented
        And I select the link Target 421
        Then I see the following links being bolded
            | Instrumented |
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | Target 421   |
            | field1       |

    Scenario: Should see going deep down into nested routes in SSR bold links
        When I select the link Instrumented
        And I select the link Target 421
        And I reload the page
        Then I see the following links being bolded
            | Instrumented |
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | Target 421   |
            | field1       |

    Scenario: Going deep down navigate around nested links bold correctly
        When I select the link Instrumented
        And I select the link Target 421
        And I select the link Inspect path2/field3
        Then I see the following links being bolded
            | Instrumented |
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | field3       |
        And I see the following links not being bolded
            | Target 421   |
            | field1       |

    Scenario: Going deep down navigate around nested links bold correctly, SSR
        When I select the link Instrumented
        And I select the link Target 421
        And I select the link Inspect path2/field3
        And I reload the page
        Then I see the following links being bolded
            | Instrumented |
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | field3       |
        And I see the following links not being bolded
            | Target 421   |
            | field1       |

    Scenario: Going deep down back out nested routes reset bolded states
        When I select the link Instrumented
        And I select the link Target 421
        And I select the link Counters
        Then I see the following links being bolded
            | Instrumented |
            | Counters     |
        And I see the following links not being bolded
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | Target 421   |

    Scenario: Going deep down back out nested routes reset bolded states, SSR
        When I select the link Instrumented
        And I select the link Target 421
        And I select the link Counters
        And I reload the page
        Then I see the following links being bolded
            | Instrumented |
            | Counters     |
        And I see the following links not being bolded
            | Item Listing |
            | Target 4##   |
            | Target 42#   |
            | Target 421   |
