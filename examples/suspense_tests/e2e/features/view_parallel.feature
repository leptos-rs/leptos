@view_parallel
Feature: View Parallel

    Background:

        Given I see the app

    @view_parallel-title
    Scenario Outline: Should see the page title
        Given I select the mode <Mode>
        When I select the component Parallel
        Then I see the page title is <Mode>

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_parallel-one
    Scenario Outline: Should see the one second message
        Given I select the mode <Mode>
        When I select the component Parallel
        Then I see the one second message is One Second: Loaded 1!

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_parallel-two
    Scenario Outline: Should see the two second message
        Given I select the mode <Mode>
        When I select the component Parallel
        Then I see the two second message is Two Second: Loaded 2!

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |