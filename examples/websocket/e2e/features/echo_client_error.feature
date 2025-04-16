@echo_client_error
Feature: Echo Client Error

    Background:
        Given I see the app

    @echo_client_error-see-fifth-input-error
    Scenario: Should see the client error
        Given I add a text as abcde
        Then I see the label of the input is Error(ServerFnErrorWrapper(Registration("Error generated from client")))
