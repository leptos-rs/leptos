window.addEventListener("click", async (ev) => {
	// confirm that this is an <a> that meets our requirements
	if (
		ev.defaultPrevented ||
		ev.button !== 0 ||
		ev.metaKey ||
		ev.altKey ||
		ev.ctrlKey ||
		ev.shiftKey
	      )
        return;

      /** @type HTMLAnchorElement | undefined;*/
      const a = ev
        .composedPath()
        .find(el => el instanceof Node && el.nodeName.toUpperCase() === "A");

	if (!a) return;

     const svg = a.namespaceURI === "http://www.w3.org/2000/svg";
	const href = svg ? a.href.baseVal : a.href;
      const target = svg ? a.target.baseVal : a.target;
      if (target || (!href && !a.hasAttribute("state"))) return;

      const rel = (a.getAttribute("rel") || "").split(/\s+/);
      if (a.hasAttribute("download") || (rel && rel.includes("external"))) return;

      const url = svg ? new URL(href, document.baseURI) : new URL(href);
      if (
        url.origin !== window.location.origin // ||  
	      // TODO base
        //(basePath && url.pathname && !url.pathname.toLowerCase().startsWith(basePath.toLowerCase()))
      )
        return;

	ev.preventDefault();

	// fetch the new page
	const resp = await fetch(url);
	    const htmlString = await resp.text();

    // Use DOMParser to parse the HTML string
    const parser = new DOMParser();
	// TODO parse from the request stream instead?
    const doc = parser.parseFromString(htmlString, 'text/html');

	// The 'doc' variable now contains the parsed DOM 
	const transition = document.startViewTransition(async () => {
		const oldDocWalker = document.createTreeWalker(document);
		const newDocWalker = doc.createTreeWalker(doc);
		let oldNode = oldDocWalker.currentNode;
		let newNode = newDocWalker.currentNode;
		while(oldDocWalker.nextNode() && newDocWalker.nextNode()) {
			oldNode = oldDocWalker.currentNode;
			newNode = newDocWalker.currentNode;
			// if the nodes are different, we need to replace the old with the new
			// because of the typed view tree, this should never actually happen
			if (oldNode.nodeType !== newNode.nodeType) {
				oldNode.replaceWith(newNode);
			}
			// if it's a text node, just update the text with the new text
			else if (oldNode.nodeType === Node.TEXT_NODE) {
				oldNode.textContent = newNode.textContent;
			}
			// if it's an element, replace if it's a different tag, or update attributes
			else if (oldNode.nodeType === Node.ELEMENT_NODE) {
				/** @type Element */
				const oldEl = oldNode;
				/** @type Element */
				const newEl = newNode;
				if (oldEl.tagName !== newEl.tagName) {
					oldEl.replaceWith(newEl);
				}
				else {
					for(const attr of newEl.attributes) {
						oldEl.setAttribute(attr.name, attr.value);
					}
				}
			}
			// we use comment "branch marker" nodes to distinguish between different branches in the statically-typed view tree
			// if one of these marker is hit, then there are two options
			// 1) it's the same branch, and we just keep walking until the end 
			// 2) it's a different branch, in which case the old can be replaced with the new wholesale
			else if (oldNode.nodeType === Node.COMMENT_NODE) {
				const oldText = oldNode.textContent;
				const newText = newNode.textContent;
				if(oldText.startsWith("bo") && newText !== oldText) {
					oldDocWalker.nextNode();
					newDocWalker.nextNode();
					const oldRange = new Range();
					const newRange = new Range();
					let oldBranches = 1;
					let newBranches = 1;
					while(oldBranches > 0 && newBranches > 0) {
						if(oldDocWalker.nextNode() && newDocWalker.nextNode()) {
						console.log(oldDocWalker.currentNode, newDocWalker.currentNode);
							if(oldDocWalker.currentNode.nodeType === Node.COMMENT_NODE) {
								if(oldDocWalker.currentNode.textContent.startsWith("bo")) {
									oldBranches += 1;
								} else if(oldDocWalker.currentNode.textContent.startsWith("bc")) {

									oldBranches -= 1;
								}
							}
							if(newDocWalker.currentNode.nodeType === Node.COMMENT_NODE) {
								if(newDocWalker.currentNode.textContent.startsWith("bo")) {
									newBranches += 1;
								} else if(newDocWalker.currentNode.textContent.startsWith("bc")) {

									newBranches -= 1;
								}
							}
						}
					}

					try {
						oldRange.setStartAfter(oldNode);
						oldRange.setEndBefore(oldDocWalker.currentNode);
						newRange.setStartAfter(newNode);
						newRange.setEndBefore(newDocWalker.currentNode);
						const newContents = newRange.extractContents();
						oldRange.deleteContents();
						oldRange.insertNode(newContents);
						oldNode.replaceWith(newNode);
						oldDocWalker.currentNode.replaceWith(newDocWalker.currentNode);
					} catch (e) {
						console.error(e);
					}
				} 			}
		}
	});
	await transition;
	window.history.pushState(undefined, null, url);
});

