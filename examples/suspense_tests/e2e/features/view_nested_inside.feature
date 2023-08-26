@view_nested_inside
Feature: View Nested Inside

    Background:

        Given I see the app

    @view_nested_inside-title
    Scenario Outline: Should see the page title
        Given I select the mode <Mode>
        When I select the component Nested (resource created inside)
        Then I see the page title is <Mode>

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_nested_inside-one
    Scenario Outline: Should see the one second message
        Given I select the mode <Mode>
        When I select the component Nested (resource created inside)
        Then I see the one second message is One Second: Loaded 1!

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_nested_inside-two
    Scenario Outline: Should see the two second message
        Given I select the mode <Mode>
        When I select the component Nested (resource created inside)
        Then I see the two second message is Loaded 2 (created inside first suspense)!: Ok(())

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |