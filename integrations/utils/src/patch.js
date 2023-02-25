function patch(json) {
	const views = JSON.parse(json);
	for (view of views) {
		const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT),
			open = `leptos-view|${view.id}|open`,
			close = `leptos-view|${view.id}|close`;
		let start, end;
		while (walker.nextNode()) {
			if (walker.currentNode.textContent == open) {
				start = walker.currentNode;
			} else if (walker.currentNode.textContent == close) {
				end = walker.currentNode;
				break;
			}
		}
		const firstNode = start.nextSibling;
		applyPatch(firstNode, view.template);
	}

	function applyPatch(element, template) {
		console.log("> applyPatch", element, template)
		// patch element against element 
		if (template.Element && element.nodeType == Node.ELEMENT_NODE) {
			// patch attributes
			for (const [name, value] of template.Element.attrs) {
				if (value.Static) {
					element.setAttribute(name, value.Static);
				}
			}
		}

		// patch children

		// TODO
		// 1. Handle inserting a child
		// 2. Handle removing a child
		// 3. Handle moving a child
		// 4. Actually send server/client reload signals
		// 5. Refresh cargo-leptos understanding of what we've got after we send a reload

		let currentActualChild;
		for (const child of template.Element.children) {
			// skip over dynamic children
			if (child == "DynChild") {
				let nextChild = currentActualChild
					? currentActualChild.nextSibling
					: element.firstChild;
				// skip over everything until this <DynChild> is done
				if (nextChild.textContent.trim() == "<DynChild>") {
					let nesting = 1;
					while (nesting > 0) {
						nextChild = nextChild.nextSibling;
						if (nextChild.textContent.trim() == "<DynChild>") {
							nesting++;
						} else if (nextChild.textContent.trim() == "</DynChild>") {
							nesting--;
						}
					}
					currentActualChild = nextChild;
					console.log("final currentActualChild = ", currentActualChild);
				} else {
					console.log("didn't match");
				}
			}
			// handle text children 
			else if (child.Text) {
				const nextChild = currentActualChild
					? currentActualChild.nextSibling
					: element.firstChild;
				currentActualChild = nextChild;
				if (currentActualChild.nodeType == Node.TEXT_NODE) {
					currentActualChild.textContent = child.Text;
				}
			}
			// handle element children 
			else if (child.Element) {
				const nextChild = currentActualChild
					? currentActualChild.nextElementSibling
					: element.firstElementChild;
				currentActualChild = nextChild;
				applyPatch(currentActualChild, child);
			}
		}
	}
}
