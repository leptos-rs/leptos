let NAVIGATION = 0;

window.addEventListener("click", async (ev) => {
	const req = clickToReq(ev);
	if(!req) {
		return;
	}

	ev.preventDefault();
	await navigateToPage(req, true);
});

window.addEventListener("popstate", async (ev) => {
	const req = new Request(window.location);
	ev.preventDefault();
	await navigateToPage(req, true, true);
});

window.addEventListener("submit", async (ev) => {
	if (ev.defaultPrevented) {
		return;
	}
		
	const req = submitToReq(ev);
	if(!req) {
		return;
	}

	ev.preventDefault();
	await navigateToPage(req, true);
});

async function navigateToPage(
	/** @type Request */
	req, 
	/** @type bool */
	useViewTransition, 
	/** @type bool */
	replace
) {
	NAVIGATION += 1;
	const currentNav = NAVIGATION;

	// add a custom header to indicate that we're on a subsequent navigation 
	req.headers.append("Islands-Router", "true");

	// fetch the new page
	const resp = await fetch(req);
	const redirected = resp.redirected;
	const htmlString = await resp.text();

	if(NAVIGATION === currentNav) {
		// The 'doc' variable now contains the parsed DOM
		const transition = async () => {
			try {
				diffPages(htmlString);
				for(const island of document.querySelectorAll("leptos-island")) {
					if(!island.$$hydrated) {
						__hydrateIsland(island, island.dataset.component);
						island.$$hydrated = true;
					}
				}
			} catch(e) {
				console.error(e);
			}
		};
		// Not all browsers support startViewTransition; see https://caniuse.com/?search=startViewTransition
		if (useViewTransition && document.startViewTransition) {
			await document.startViewTransition(transition);
		} else {
			await transition()
		}

		const url = redirected ? resp.url : req.url;

		if(replace) {
			window.history.replaceState(undefined, null, url);
		} else {
			window.history.pushState(undefined, null, url);
		}
	}
}

function clickToReq(ev) {
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
      if (a.hasAttribute("download") || (rel?.includes("external"))) return;

      const url = svg ? new URL(href, document.baseURI) : new URL(href);
      if (
        url.origin !== window.location.origin // ||  
	      // TODO base
        //(basePath && url.pathname && !url.pathname.toLowerCase().startsWith(basePath.toLowerCase()))
      )
        return; 

      return new Request(url);
}

function submitToReq(ev) {
	event.preventDefault();

	const target = ev.target;
	/** @type HTMLFormElement */
	let form;
	if(target instanceof HTMLFormElement) {
		form = target;
	} else {
		if(!target.form) {
			return;
		}
		form = target.form;
	}

	const method = form.method.toUpperCase();
	if(method !== "GET" && method !== "POST") {
		return;
	}

	const url = new URL(form.action);
	let path = url.pathname;
	const requestInit = {};
	const data = new FormData(form);

	const params = new URLSearchParams();
	for (const [key, value] of data.entries()) {
		params.append(key, value);
	}

	requestInit.headers = { 
		Accept: "text/html"
	};
	if(method === "GET") {
		path += `?${params.toString()}`;
	}
	else {
		requestInit.method = "POST";
		requestInit.body = params; 
	}

	return new Request(
		path,
		requestInit
	);
}


function diffPages(htmlString) {
	// Use DOMParser to parse the HTML string
	const parser = new DOMParser();
	const doc = parser.parseFromString(htmlString, 'text/html');

	diffRange(document, document, doc, doc);
}

function diffRange(oldDocument, oldRoot, newDocument, newRoot, oldEnd, newEnd) {
	const oldDocWalker = oldDocument.createTreeWalker(oldRoot);
	const newDocWalker = newDocument.createTreeWalker(newRoot);
	let oldNode = oldDocWalker.currentNode;
	let newNode = newDocWalker.currentNode;

	while (oldDocWalker.nextNode() && newDocWalker.nextNode()) {
		oldNode = oldDocWalker.currentNode;
		newNode = newDocWalker.currentNode;

		if (oldNode == oldEnd || newNode == newEnd) {
			break;
		}

		// if the nodes are different, we need to replace the old with the new
		// because of the typed view tree, this should never actually happen
		if (oldNode.nodeType !== newNode.nodeType) {
			oldNode.replaceWith(newNode);
		}
		// if it's a text node, just update the text with the new text
		else if (oldNode.nodeType === Node.TEXT_NODE) {
			oldNode.textContent = newNode.textContent;
		}
		// islands should not be diffed on the client, because we do not want to overwrite client-side state 
		// but their children should be diffed still, because they could contain new server content
		else if (oldNode.nodeType === Node.ELEMENT_NODE && oldNode.tagName === "LEPTOS-ISLAND") {
			// TODO: diff the leptos-children 

			// skip over leptos-island otherwise
			oldDocWalker.nextSibling();
			newDocWalker.nextSibling();
		}
		// if it's an element, replace if it's a different tag, or update attributes
		else if (oldNode.nodeType === Node.ELEMENT_NODE) {
			diffElement(oldNode, newNode);
		}
		// we use comment "branch marker" nodes to distinguish between different branches in the statically-typed view tree
		// if one of these marker is hit, then there are two options
		// 1) it's the same branch, and we just keep walking until the end 
		// 2) it's a different branch, in which case the old can be replaced with the new wholesale
		else if (oldNode.nodeType === Node.COMMENT_NODE) {
			const oldText = oldNode.textContent;
			const newText = newNode.textContent;
			if(oldText.startsWith("bo-for")) {
				replaceFor(oldDocument, oldDocWalker, newDocument, newDocWalker, oldNode, newNode);
			}
			else if (oldText.startsWith("bo-item")) {
				// skip, this means we're diffing a new item within a For
			}
			else if(oldText.startsWith("bo") && newText !== oldText) {
				replaceBranch(oldDocWalker, newDocWalker, oldNode, newNode);
			}
		}
	}
}

function replaceFor(oldDocument, oldDocWalker, newDocument, newDocWalker, oldNode, newNode) {
	oldDocWalker.nextNode();
	newDocWalker.nextNode();
	const oldRange = new Range();
	const newRange = new Range();
	let oldBranches = 1;
	let newBranches = 1;

	const oldKeys = {};
	const newKeys = {};

	while(oldBranches > 0) {
		const c = oldDocWalker.currentNode;
		if(c.nodeType === Node.COMMENT_NODE) {
			const t = c.textContent;
			if(t.startsWith("bo-for")) {
				oldBranches += 1;
			} else if(t.startsWith("bc-for")) {

				oldBranches -= 1;
			} else if (t.startsWith("bo-item")) {
				const k = t.replace("bo-item-", "");
				oldKeys[k] = { open: c, close: null };
			} else if (t.startsWith("bc-item")) {
				const k = t.replace("bc-item-", "");
				oldKeys[k].close = c;
			}
		}
		oldDocWalker.nextNode();
	}
	while(newBranches > 0) {
		const c = newDocWalker.currentNode;
		if(c.nodeType === Node.COMMENT_NODE) {
			const t = c.textContent;
			if(t.startsWith("bo-for")) {
				newBranches += 1;
			} else if(t.startsWith("bc-for")) {

				newBranches -= 1;
			} else if (t.startsWith("bo-item")) {
				const k = t.replace("bo-item-", "");
				newKeys[k] = { open: c, close: null };
			} else if (t.startsWith("bc-item")) {
				const k = t.replace("bc-item-", "");
				newKeys[k].close = c;
			}
		}
		newDocWalker.nextNode();
	}

	for(const key in oldKeys) {
		if(newKeys[key]) {
			const oldOne = oldKeys[key];
			const newOne = newKeys[key];
			const oldRange = new Range();
			const newRange = new Range();

			// then replace the item in the *new* list with the *old* DOM elements 
			oldRange.setStartAfter(oldOne.open);
			oldRange.setEndBefore(oldOne.close);
			newRange.setStartAfter(newOne.open);
			newRange.setEndBefore(newOne.close);
			const oldContents = oldRange.extractContents();
			const newContents = newRange.extractContents();

			// patch the *old* DOM elements with the new ones
			diffRange(oldDocument, oldContents, newDocument, newContents, oldOne.close, newOne.close);

			// then insert the old DOM elements into the new tree 
			// this means you'll end up with any new attributes or content from the server, 
			// but with any old DOM state (because they are the old elements)
			newRange.insertNode(oldContents);
			newOne.open.replaceWith(oldOne.open);
			newOne.close.replaceWith(oldOne.close);
		}
	}

	try {
		oldRange.setStartAfter(oldNode);
		oldRange.setEndBefore(oldDocWalker.currentNode);
		newRange.setStartAfter(newNode);
		newRange.setEndAfter(newDocWalker.currentNode);
		const newContents = newRange.extractContents();
		oldRange.deleteContents();
		oldRange.insertNode(newContents);
		oldNode.replaceWith(newNode);
		oldDocWalker.currentNode.replaceWith(newDocWalker.currentNode);
	} catch (e) {
		console.error(e);
	}
}

function replaceBranch(oldDocWalker, newDocWalker, oldNode, newNode) {
	oldDocWalker.nextNode();
	newDocWalker.nextNode();
	const oldRange = new Range();
	const newRange = new Range();
	let oldBranches = 1;
	let newBranches = 1;
	while(oldBranches > 0) {
		if(oldDocWalker.nextNode()) {
			if(oldDocWalker.currentNode.nodeType === Node.COMMENT_NODE) {
				if(oldDocWalker.currentNode.textContent.startsWith("bo")) {
					oldBranches += 1;
				} else if(oldDocWalker.currentNode.textContent.startsWith("bc")) {

					oldBranches -= 1;
				}
			}
		}
	}
	while(newBranches > 0) {
		if(newDocWalker.nextNode()) {
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
		newRange.setEndAfter(newDocWalker.currentNode);
		const newContents = newRange.extractContents();
		oldRange.deleteContents();
		oldRange.insertNode(newContents);
		oldNode.replaceWith(newNode);
		oldDocWalker.currentNode.replaceWith(newDocWalker.currentNode);
	} catch (e) {
		console.error(e);
	}
}

function diffElement(oldNode, newNode) {
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

for(const island of document.querySelectorAll("leptos-island")) {
	island.$$hydrated = true;
}
