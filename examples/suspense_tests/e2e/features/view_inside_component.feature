@view_inside_component
Feature: View Inside Component

    Background:

        Given I see the app

    @view_inside_component
    Scenario Outline: Should see the page title
        Given I select the mode <Mode>
        When I select the component Inside Component
        Then I see the page title is <Mode>

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_inside_component-one
    Scenario Outline: Should see the one second message
        Given I select the mode <Mode>
        When I select the component Inside Component
        Then I see the one second message is One Second: Loaded 1!

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_inside_component-inside
    Scenario Outline: Should see the inside message
        Given I select the mode <Mode>
        When I select the component Inside Component
        Then I see the inside message is Suspense inside another component should work.

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

    @view_inside_component-following
    Scenario Outline: Should see the following message
        Given I select the mode <Mode>
        When I select the component Inside Component
        Then I see the following message is Children following Suspense should hydrate properly.

        Examples:
            | Mode         |
            | Out-of-Order |
            | In-Order     |
            | Async        |

