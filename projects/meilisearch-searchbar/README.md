# Meilisearch Searchbar

This show how to integrate meilisearch with a leptos app, including a search bar and showing the results to the user.
<br><br>
We'll run meilisearch locally, as opposed to using their cloud service.
<br><br>
To get started install meilisearch into this example's root.

```sh
curl -L https://install.meilisearch.com | sh
```

Run it.

```sh
./meilisearch
```

Then set the environment variable and serve the app. I've included the address of my own local meilisearch server.
I didn't provide a password to meilisearch during my setup, and I didn't provide one in my environment variables either.
```sh
MEILISEARCH_URL=http://localhost:7700 && cargo leptos serve
```

Navigate to 127.0.0.1:3000 and start typing in popular American company names. (Boeing, Pepsi, etc)

## Thoughts, Feedback, Criticism, Comments?
Send me any of the above, I'm @sjud on leptos discord. I'm always looking to improve and make these projects more helpful for the community. So please let me know how I can do that. Thanks!