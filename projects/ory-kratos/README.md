# Leptos Ory Kratos Integration (With Axum)
This repo used [start-axum-workspace](https://github.com/leptos-rs/start-axum-workspace/) as a base.

## How to run the example.

Run in different terminal windows (for the best result)

```sh
cargo leptos serve
```

```sh
docker compose up
```

```sh
cargo test --test app_suite
```

This will run our server, set up our compose file (MailCrab, Ory Kratos, Ory Ketos) and run the test suite that walks through logging in, registration, verification etc.

The e2e testing uses [chromiumoxide](https://crates.io/crates/chromiumoxide) and does things like monitor network requests, console messages, take screenshots during the flow and produces them when any of our feature tests fail. This can be a helpful starting point in debugging. Currently it just prints the output into files in the e2e directory but it could be modified to pipe them somewhere like a tool to help with the development process.


## High Level Overview

Our project runs a leptos server alongside various Ory Kratos. Kratos provides identification, and we use it when registering users, and credentialing them.
<br>
A normal flow would look something like:<br>
<ul>
<li>
I go to the homepage,I click register
</li>
</li>
I am redirected to the register page, the register page isn't hardcoded but is rendered by parsing the UI data structure given by Ory Kratos. The visible portions correspond to the fields we've set in our ./kratos/email.schema.json schema file, but it includes
hidden fields (i.e a CSRF token to prevent CSRF). This project includes unstyled parsing code for the UI data structure.
</li>
<li>
I sign up with an email and password
</li>
<li>
Our leptos server will intercept the form data and then pass it on to the ory kratos service.
</li>
<li>
Ory Kratos validates those inputs given the validation criteria ./kratos/email.schema.json schema file
</li>
<li>
Ory Kratos then verifies me by sending me an email.
</li>
<li>
In this example we catch the email with an instance of mailcrab (an email server for testing purposes we run in our docker compose)
. You can use mailcrab locally 127.0.0.1:1080
</li>
<li>
I look inside the email, I see a code and a link where I will input the code.
</li>
<li>
I click through and input the code, and I am verified.
</li>
<li>
When I go to the login page, it's rendered based on the same method as the registration page. I.e Kratos sends a UI data structure which is parsed into the UI we show the user.
</li>
<li>
I use my password and email on the login page to login.
</li>
<li>
Again, Our leptos server acts as the inbetween between the client and the Ory Kratos service. There were some pecularities between the CSRF token being set in the headers (which Ory Kratos updates with every step in the flow), SSR, and having the client communicate directly with Ory Kratos which lead me to use this approach where our server is the intermediary between the client and Ory Kratos.
</li>
<li>
Ory Kratos is session based, so after it recieves valid login credentials it creates a session and returns the session token. The session token is passed via cookies with every future request. All this does is establish the identity of the caller, to perform authentication we need a way to establish permissions given an individuals identity and how that relates to the content on the website. In this example I just use tables in the database but this example could be extended to use Ory Ketos, with is to Authorization a Ory Kratos is to Identification.
</li>
</ul>

When given bad input in a field, Ory Kratos issues a new render UI data structure with error messages and we rerender the login page.

## With regards to Ory Oathkeeper And Ory Ketos.

Ory Oathkeeper is a reverse proxy that sits between your server and the client, it takes the session token, looks to see what is being requested in the request and then checks the configuration files of your Ory Services to see if such a thing is allowed. It will communicate with the Ory services on your behalf and then pass on the authorized request to the appropriate location or reject it otherwise.
<br>
Ory Ketos is the authorization part of the Ory suite, Ory Kratos simplies identifies the user (this is often conflated with authorization but authorization is different). Authorization is the process of after having confirmed a user's identity provisioning services based on some permission structure. I.e Role Based Authorization, Document based permissions, etc. Ory Ketos uses a similar configuration file based set up to Ory Kratos.
<br>
Instead of either of those, in this example we use an extractor to extract the session cookie and verify it with our kratos service and then perform our own checks. This is simpler to set up, more inutitive, and thus better for smaller projects. Identification is complicated, and it's nice to have it be modularized for whatever app we are building. This will save a lot of time when building multiple apps. The actual provisioning of services for most apps is much simpler, i.e database lookup tied to identification and some logic checks. Is the user preiumum? How much have they used the API compared to the maximum? Using Ory Kratos can reduce complexity and decrease your time to market, especially over multiple attempts.
<br>
In production you'd have a virtual private server and you'd serve your leptos server behind Nginx, Nginx routes the calls to the Leptos Server and never to our Ory Kratos. Our Rust server handles all the communication between the client and Ory services. This is simpler from an implementation perspective then including Ory Oathkeeper and Ory Ketos. Ory Kratos/Ketos presume all api calls they recieve are valid by default, so it's best not to expose them at all to any traffic from the outside world. And when building our leptos app we'll have a clear idea about when and how these services are being communicated with when our service acts as the intermediary.

## How this project is tested

We use Gherkin feature files to describe the behavior of the application. We use [cucumber](https://docs.rs/cucumber/latest/cucumber/) as our test harness and match the feature files to [chromiumoxide](https://docs.rs/chromiumoxide/latest/chromiumoxide/) code to drive a local chromium application. I'm using e2e testing mostly to confirm that the service provides the value to the user, in this case just authorization testing. And that, that value proposition doesn't break when we change some middleware code that touches everything etc.
<br>
The `ids` crate includes a list of static strings that we'll use in our chromiumoxide lookups and our frontend to make our testing as smooth as possible. There are other ways to do this, such as find by text, which would find the "Sign Up" text and click it etc. So these tests don't assert anything with regards to presentation, just functionality.

## How to use mkcert to get a locally signed certificate (and why)
We need to use https because we are sending cookies with the `Secure;` flag, cookies with the Secure flag can't be used 
unless delivered over https. Since we're using chromedriver for e2e testing let's use mkcert to create a cert that will allow 
https://127.0.0.1:3000/ to be a valid url.
Install mkcert and then

```sh
mkcert -install localhost 127.0.0.1 ::1
```

Copy your cert.pem, key.pem and rootCA.pem into this crate's root.


## Thoughts, Feedback, Criticism, Comments?
Send me any of the above, I'm @sjud on leptos discord. I'm always looking to improve and make these projects more helpful for the community. So please let me know how I can do that. Thanks!
