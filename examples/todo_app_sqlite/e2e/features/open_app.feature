@open_app
Feature: Open App

  @open_app-title
  Scenario: Should see the home page title
    When I open the app
    Then I see the page title is My Tasks

  @open_app-label
  Scenario: Should see the input label
    When I open the app
    Then I see the label of the input is Add a Todo