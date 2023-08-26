@view_single
Feature: View Single

    Background:

        Given I see the app

    @view_single-title
    Scenario Outline: Should see the page title
        Given I select the mode <Mode>
        When I select the component Single
        Then I see the page title is <Mode>

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_single-one
    Scenario Outline: Should see the one second message
        Given I select the mode <Mode>
        When I select the component Single
        Then I see the one second message is One Second: Loaded 1!

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_single-following
    Scenario Outline: Should see the following message
        Given I select the mode <Mode>
        When I select the component Single
        Then I see the following message is Children following Suspense should hydrate properly.

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |
