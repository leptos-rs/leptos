@delete_todo
Feature: Delete Todo

    Background:
        Given I see the app

    Rule: Remove todo

        @delete_todo-last
        Scenario: Should not see the deleted todo
            Given I add a todo titled Buy Blueberry Yogurt
            And I add a todo titled Buy Cottage Cheese
            When I delete the last todo
            Then I see the last todo is Buy Blueberry Yogurt

    Rule: See empty list message

        @delete-todo-message
        Scenario: Should see the empty list message
            Given I open the app
            When I empty the todo list
            Then I see the empty list message is No tasks were found.