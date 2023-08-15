@open_app
Feature: Open App

  @open_app-title
  Scenario: Should see the initial page title
    When I open the app
    Then I see the page title is Out-of-Order