@open_app
Feature: Open App

  @open_app-title
  Scenario: Should see the home page title
    When I open the app
    Then I see the page title is Simple Echo WebSocket Communication
