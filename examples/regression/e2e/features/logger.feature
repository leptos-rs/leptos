@check_logger
Feature: The simple logger.

    Scenario: Visiting readme should have no log messages.
        Given I see the app
        When I access (What is this)
        Then I see 0 log messages

    Scenario: Visiting readme and using the example link should log a message.
        Given I see the app
        When I access (What is this)
        And I select the link example link
        Then I counted 1 log message
        And I find the following being the most recent log messages
            | Hello world! |

    Scenario: Visiting readme and generate multiple log messages
        Given I see the app
        When I access (What is this)
        And I select the following links
            | example link |
            | example link |
            | other link   |
            | other link   |
            | example link |
        Then I counted 5 log message
        And I find the following being the most recent log messages
            | Something else. |
            | Something else. |
            | Hello world!    |
