@logout
Feature: Logout

    As a user
    I want to log out after registering
    So that I can test to see if login after registering works.

    
    Scenario:logout
        Given I am on the registration page
        And I see the registration form
        And I enter valid credentials
        And I check my email for the verification link and code
        And I copy the code onto the verification link page
        And I click login
        And I re-enter valid credentials
        When I click logout
        Then I am logged out