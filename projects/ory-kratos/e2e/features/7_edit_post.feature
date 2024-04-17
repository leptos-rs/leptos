@edit-post
Feature: Edit-Post

    As a user
    I want to add an editor to my post
    So that my bestie can improve my EXAMPLE CONTENT.

    Background:
        Given I am on the registration page
        And I see the registration form
        And I enter valid other credentials
        And I check my other email for the verification link and code
        And I copy the code onto the verification link page
        And I am on the registration page
        And I see the registration form
        And I enter valid credentials
        And I check my email for the verification link and code
        And I copy the code onto the verification link page

    Scenario: add_editor_as_owner_and_edit_post
        Given I am on the homepage
        And I click login
        And I re-enter valid credentials
        And I add example post
        And I click show post list
        And I see example content posted
        When I add other email as editor
        And I logout
        And I click login
        And I re-enter other valid credentials
        And I click show post list
        And I see example content posted
        And I edit example post
        And I click show post list
        Then I see my new content posted
        And I don't see old content

    Scenario: add_editor_as_other
        Given I am on the homepage
        And I click login
        And I re-enter valid credentials
        And I add example post
        And I click show post list
        And I see example content posted
        When I add other email as editor
        And I logout
        And I click login
        And I re-enter other valid credentials
        And I click show post list
        And I see example content posted
        And I add other email as editor
        Then I see error
