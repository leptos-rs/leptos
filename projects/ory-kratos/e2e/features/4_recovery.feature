@recovery
Feature: Recovery

    As a user
    I want to recovery my email
    So that I can test to see if recovery after registering works.

    
    Scenario:recovery
        # lol and and and and and and...
        Given I am on the registration page
        And I see the registration form
        And I enter valid credentials
        And I check my email for the verification link and code
        And I copy the code onto the verification link page
        And I click login
        And I re-enter valid credentials
        And I click logout
        And I click recover email
        And I submit valid recovery email
        And I check my email for recovery link and code
        When I copy the code onto the recovery link page
        Then I am on the settings page