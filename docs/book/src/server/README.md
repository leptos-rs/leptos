# Working with the Server

The previous section described the process of server-side rendering, using the server to generate an HTML version of the page that will become interactive in the browser. So far, everything has been “isomorphic”; in other words, your app has had the “same (_iso_) shape (_morphe_)” on the client and the server.

But a server can do a lot more than just render HTML! In fact, a server can do a whole bunch of things your browser _can’t,_ like reading from and writing to a SQL database.

If you’re used to building JavaScript frontend apps, you’re probably used to calling out to some kind of REST API to do this sort of server work. If you’re used to building sites with PHP or Python or Ruby (or Java or C# or...), this server-side work is your bread and butter, and it’s the client-side interactivity that tends to be an afterthought.

With Leptos, you can do both: not only in the same language, not only sharing the same types, but even in the same files!

This section will talk about how to build the uniquely-server-side parts of your application.
