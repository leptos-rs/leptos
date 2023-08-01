@open_app
Feature: Open App

  Rule: See home page title

    @open_app-title
    Scenario: Should see the title of the home page
      When I open the app
      Then I see the page title is My Tasks

  Rule: See todo textbox label

    @open_app-label
    Scenario: Should see the label of the todo textbox
      When I open the app
      Then I see the label of the input is Add a Todo

  Rule: See empty list message

    @open_app-message
    Scenario: Should see the empty list message
      Given I open the app
      When I empty the todo list
      Then I see the empty list message is No tasks were found.