# Parent Child Example

 This example highlights four different ways that child components can communicate with their parent:
1. <ButtonA/>: passing a WriteSignal as one of the child component props,
    for the child component to write into and the parent to read
2. <ButtonB/>: passing a closure as one of the child component props, for
    the child component to call
3.  <ButtonC/>: adding a simple event listener on the child component itself
4.  <ButtonD/>: providing a context that is used in the component (rather than prop drilling)

## Client Side Rendering
To run it as a Client Side App, you can issue  `trunk serve --open` in the root. This will build the entire
app into one CSR bundle

