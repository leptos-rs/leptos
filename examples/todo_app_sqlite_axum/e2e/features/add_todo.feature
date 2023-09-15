@add_todo
Feature: Add Todo

    Background:
        Given I see the app

    @add_todo-see
    Scenario: Should see the todo
        Given I set the todo as Buy Bread
        When I click the Add button
        Then I see the todo named Buy Bread

    @add_todo-style
    Scenario: Should see the pending todo
        When I add a todo as Buy Oranges
        Then I see the pending todo
