@add_todo
Feature: Add Todo

    Background:
        Given I see the app

    Rule: See new todo

        Scenario: Should see the todo at the end of the list
            Given I set the todo as Buy Bread
            When I click the Add button
            Then I see the last todo is Buy Bread