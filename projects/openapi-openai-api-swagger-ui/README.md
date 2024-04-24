#OpenAPI Swagger-Ui OpenAI GPT

This example shows how to document server functions via OpenAPI schema generated using Utoipa and serve the swagger ui via /swagger-ui endpoint. More than that, this example shows how to take said OpenAPI spec and turn it into a function list to feed to OpenAI's chat completion endpoint to generate the JSON values to feed back into our server functions.

The example shows an input and if you tell it to do something that is covered, say hello, or generate a list of names it will do that. 

To use the AI part of this project provide your openAPI key in an environment variable when running cargo leptos.

```sh
OPENAI_API_KEY=my_secret_key cargo leptos serve
```


## Thoughts, Feedback, Criticism, Comments?
Send me any of the above, I'm @sjud on leptos discord. I'm always looking to improve and make these projects more helpful for the community. So please let me know how I can do that. Thanks!