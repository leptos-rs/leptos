@add-post
Feature: Add-post

    As a user
    I want to add a post
    So that I can share my EXAMPLE DATA with the world!

    Background:
        Given I am on the homepage
        And I clear cookies
    
  

    Scenario: add_post_logged_in
        Given I am on the registration page
        And I see the registration form
        And I enter valid credentials
        And I check my email for the verification link and code
        And I copy the code onto the verification link page
        And I click login
        And I re-enter valid credentials
        When I add example post
        And I click show post list
        Then I see example content posted

      Scenario: add_post_logged_out
        Given I am logged out
        When I add example post
        Then I see error