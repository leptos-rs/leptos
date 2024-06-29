@delete_todo
Feature: Delete Todo

    Background:
        Given I see the app

    @serial
    @delete_todo-remove
    Scenario: Should not see the deleted todo
        Given I add a todo as Buy Yogurt
        When I delete the todo named Buy Yogurt
        Then I do not see the todo named Buy Yogurt

    @serial
    @delete_todo-message
    Scenario: Should see the empty list message
        When I empty the todo list
        Then I see the empty list message is No tasks were found.