# `<Transition/>`

You’ll notice in the `<Suspense/>` example that if you keep reloading the data, it keeps flickering back to `"Loading..."`. Sometimes this is fine. For other times, there’s [`<Transition/>`](https://docs.rs/leptos/latest/leptos/fn.Suspense.html).

`<Transition/>` behaves exactly the same as `<Suspense/>`, but instead of falling back every time, it only shows the fallback the first time. On all subsequent loads, it continues showing the old data until the new data are ready. This can be really handy to prevent the flickering effect, and to allow users to continue interacting with your application.

This example shows how you can create a simple tabbed contact list with `<Transition/>`. When you select a new tab, it continues showing the current contact until the new data loads. This can be a much better user experience than constantly falling back to a loading message.

<iframe src="https://codesandbox.io/p/sandbox/12-transition-sn38sd?selection=%5B%7B%22endColumn%22%3A15%2C%22endLineNumber%22%3A2%2C%22startColumn%22%3A15%2C%22startLineNumber%22%3A2%7D%5D&file=%2Fsrc%2Fmain.rs" width="100%" height="1000px"></iframe>
