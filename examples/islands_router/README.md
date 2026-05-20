# Work in Progress

This example is something I wrote on a long layover in the Orlando airport in July. (It was really hot!)

It is the culmination of a couple years of thinking and working toward being able to do this, which you can see 
described pretty well in the pinned roadmap issue (#1830) and its discussion of different modes of client-side
routing when you use islands.

This uses *only* server rendering, with no actual islands, but still maintains client-side state across page navigations.
It does this by building on the fact that we now have a statically-typed view tree to do pretty smart updates with 
new HTML from the client, with extremely minimal diffing.
