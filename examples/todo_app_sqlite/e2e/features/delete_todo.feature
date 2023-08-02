@delete_todo
Feature: Delete Todo

    Background:
        Given I see the app

    @delete_todo-remove
    Scenario: Should not see the deleted todo
        Given I add a todo as Buy Yogurt
        When I delete the todo named Buy Yogurt
        Then I do not see the todo named Buy Yogurt

    @delete_todo-message
    Scenario: Should see the empty list message
        Given I open the app
        When I empty the todo list
        Then I see the empty list message is No tasks were found.