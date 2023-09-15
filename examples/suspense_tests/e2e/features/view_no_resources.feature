@view_no_resources
Feature: view No Resources

    Background:

        Given I see the app

    @view_no_resources-title
    Scenario Outline: Should see the page title
        Given I select the mode <Mode>
        When I select the component No Resources
        Then I see the page title is <Mode>

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_no_resources-another
    Scenario Outline: Should see the inside message
        Given I select the mode <Mode>
        When I select the component No Resources
        Then I see the inside message is Children inside Suspense should hydrate properly.

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_no_resources-following
    Scenario Outline: Should see the following message
        Given I select the mode <Mode>
        When I select the component No Resources
        Then I see the following message is Children following Suspense should hydrate properly.

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |
