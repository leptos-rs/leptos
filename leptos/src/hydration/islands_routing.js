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
			skipSubtree(oldDocWalker);
			skipSubtree(newDocWalker);
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

function skipSubtree(walker) {
	let node = walker.currentNode;
	while(node.lastChild) {
		node = node.lastChild;
	}
	walker.currentNode = node;
}

function collectForItems(walker) {
	const items = {};
	let depth = 1;
	while(walker.nextNode()) {
		const c = walker.currentNode;
		if(c.nodeType !== Node.COMMENT_NODE) {
			continue;
		}
		const t = c.textContent;
		if(t.startsWith("bo-for")) {
			depth += 1;
		} else if(t.startsWith("bc-for")) {
			depth -= 1;
			if(depth === 0) {
				break;
			}
		} else if(depth === 1 && t.startsWith("bo-item-")) {
			items[t.slice("bo-item-".length)] = { open: c, close: null };
		} else if(depth === 1 && t.startsWith("bc-item-")) {
			const item = items[t.slice("bc-item-".length)];
			if(item) {
				item.close = c;
			}
		}
	}
	return items;
}

function replaceFor(oldDocument, oldDocWalker, newDocument, newDocWalker, oldNode, newNode) {
	const oldKeys = collectForItems(oldDocWalker);
	const newKeys = collectForItems(newDocWalker);
	// both walkers now rest on their closing `bc-for` markers
	const oldClose = oldDocWalker.currentNode;
	const newClose = newDocWalker.currentNode;

	try {
		for(const key in newKeys) {
			const oldOne = oldKeys[key];
			const newOne = newKeys[key];
			if(!oldOne || !oldOne.close || !newOne.close) {
				continue;
			}

			// take the *old* DOM elements for this item out of the page and
			// patch them with the new content from the server
			const oldRange = new Range();
			const newRange = new Range();
			oldRange.setStartAfter(oldOne.open);
			oldRange.setEndBefore(oldOne.close);
			newRange.setStartAfter(newOne.open);
			newRange.setEndBefore(newOne.close);
			const oldContents = oldRange.extractContents();
			const newContents = newRange.extractContents();
			diffRange(oldDocument, oldContents, newDocument, newContents);

			// then put the old DOM elements in place of the item in the *new* list
			// this means you'll end up with any new attributes or content from the server,
			// but with any old DOM state (because they are the old elements)
			newOne.close.before(oldContents);
		}

		// finally, replace the old list's contents with the new list's
		const oldRange = new Range();
		const newRange = new Range();
		oldRange.setStartAfter(oldNode);
		oldRange.setEndBefore(oldClose);
		newRange.setStartAfter(newNode);
		newRange.setEndBefore(newClose);
		const newContents = newRange.extractContents();
		oldRange.deleteContents();
		oldClose.before(newContents);
	} catch (e) {
		console.error(e);
	}
}

// Advances the walker from a branch's opening marker comment to its matching
// closing marker, leaving it as the walker's currentNode. Every node after
// the opening marker is examined (including the first one: a branch may
// begin with a nested branch marker, e.g. when a view starts with a
// component that renders no element). Returns false if the walker is
// exhausted before the branch is balanced.
function seekBranchClose(walker) {
	let branches = 1;
	while(walker.nextNode()) {
		const node = walker.currentNode;
		if(node.nodeType === Node.COMMENT_NODE) {
			if(node.textContent.startsWith("bo")) {
				branches += 1;
			} else if(node.textContent.startsWith("bc")) {
				branches -= 1;
				if(branches === 0) {
					return true;
				}
			}
		}
	}
	return false;
}

function replaceBranch(oldDocWalker, newDocWalker, oldNode, newNode) {
	const oldRange = new Range();
	const newRange = new Range();
	seekBranchClose(oldDocWalker);
	seekBranchClose(newDocWalker);

	const oldClose = oldDocWalker.currentNode;
	const newClose = newDocWalker.currentNode;

	try {
		oldRange.setStartAfter(oldNode);
		oldRange.setEndBefore(oldClose);
		newRange.setStartAfter(newNode);
		newRange.setEndBefore(newClose);
		const newContents = newRange.extractContents();
		oldRange.deleteContents();
		oldRange.insertNode(newContents);
		oldNode.textContent = newNode.textContent;
		oldClose.textContent = newClose.textContent;
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
