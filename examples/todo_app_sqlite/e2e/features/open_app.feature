@open_app
Feature: Open App

  Rule: Should see the title of the home page

    Scenario: Should see the title of the home page
      When I open the app
      Then I see the page title is My Tasks