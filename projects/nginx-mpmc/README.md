# Nginx Multiple Server Multiple Client Example
This example shows how multiple clients can communicate with multiple servers while being shared over a single domain i.e localhost:80 using nginx as a reverse proxy.

### How to run this example
```sh
./run.sh 
```
Or

```sh
./run_linux.sh
```

<br>
This will boot up nginx via it's docker image mapped to port 80, and the four servers. App-1, App-2, Shared-Server-1, Shared-Server-2.
<br>
App-1, And App-2 are SSR rendering leptos servers.
<br>
If you go to localhost (you'll get App-1), and localhost/app2 (you'll get app2).
<br>
The two shared servers can be communicated with via actions and local resources, or resources (if using CSR).
<br>
`create_resource` Won't work as expected, when trying to communicate to different servers. It will instead try to run the server function on the server you are serving your server side rendered content from. This will cause errors if your server function relies on state that is not present.
<br>
When you are done with this example, run

```sh
./kill.sh
```

Casting ctrl-c multiple times won't close all the open programs.

## Thoughts, Feedback, Criticism, Comments?
Send me any of the above, I'm @sjud on leptos discord. I'm always looking to improve and make these projects more helpful for the community. So please let me know how I can do that. Thanks!