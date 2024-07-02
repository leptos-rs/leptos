Isomorphic web applications that run on the server to render HTML, then add interactivity in
the client, need to accomplish two tasks:

1. Send HTML from the server, so that the client can "hydrate" it in the browser by adding
   event listeners and setting up other interactivity.
2. Send data that was loaded on the server to the client, so that the client "hydrates" with
   the same data with which the server rendered HTML.

This crate helps with the second part of this process. It provides a [`SharedContext`] type
that allows you to store data on the server, and then extract the same data in the client.
