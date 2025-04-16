@echo_server_error
Feature: Echo Server Error

    Background:
        Given I see the app

    @echo_server_error-see-third-input-error
    Scenario: Should see the server error
        Given I add a text as abc
        Then I see the label of the input is Error(ServerFnErrorWrapper(Registration("Error generated from server")))
