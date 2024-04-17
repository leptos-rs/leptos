@register
Feature: Register

    As a user
    I want to register
    So that I can login and POST CONTENT.
    
    Scenario:register
        Given I am on the homepage
        And I click register
        And I am on the registration page
        And I see the registration form
        When I enter valid credentials
        And I check my email for the verification link and code
        And I copy the code onto the verification link page
        Then I am on the homepage