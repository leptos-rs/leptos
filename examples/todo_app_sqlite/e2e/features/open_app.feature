@open_app
Feature: Open App

  Rule: Should see the title of the home page

    Scenario: Should see the title of the home page
      When I open the app
      Then I see the page title is My Tasks

  Rule: Should see the label of the todo input

    Scenario: Should see the label of the todo input
      When I open the app
      Then I see the label of the input is Add a Todo