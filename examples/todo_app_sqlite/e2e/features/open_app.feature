@open_app
Feature: Open App

  Rule: See home page title

    Scenario: Should see the title of the home page
      When I open the app
      Then I see the page title is My Tasks

  Rule: See todo textbox label

    Scenario: Should see the label of the todo textbox
      When I open the app
      Then I see the label of the input is Add a Todo