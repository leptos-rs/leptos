function patch(json) {
	try {
		const views = JSON.parse(json);
		for ([id, patches] of views) {
			const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_COMMENT),
				open = `leptos-view|${id}|open`,
				close = `leptos-view|${id}|close`;
			let start, end;
			while (walker.nextNode()) {
				if (walker.currentNode.textContent == open) {
					start = walker.currentNode;
				} else if (walker.currentNode.textContent == close) {
					end = walker.currentNode;
					break;
				}
			}
			// build tree of current actual children
			const range = new Range();
			range.setStartAfter(start);
			range.setEndBefore(end);
			const actualChildren = buildActualChildren(start.parentElement, range);
			const actions = [];

			// build up the set of actions
			for (const patch of patches) {
				const child = childAtPath(
					actualChildren.length > 1 ? { children: actualChildren } : actualChildren[0],
					patch.path
				);
				const action = patch.action;
				if (action == "ClearChildren") {
					actions.push(() => {
						console.log("[HOT RELOAD] > ClearChildren", child.node);
						child.node.textContent = ""
					});
				} else if (action.ReplaceWith) {
					actions.push(() => {
						console.log("[HOT RELOAD] > ReplaceWith", child, action.ReplaceWith);
						const replacement = fromReplacementNode(action.ReplaceWith, actualChildren);
						if (child.node) {
							child.node.replaceWith(replacement)
						} else {
							const range = new Range();
							range.setStartAfter(child.start);
							range.setEndAfter(child.end);
							range.deleteContents();
							child.start.replaceWith(replacement);
						}
					});
				} else if (action.ChangeTagName) {
					actions.push(() => {
						console.log("[HOT RELOAD] > ChangeTagName", child.node, action.ChangeTagName);
						const newElement = document.createElement(action.ChangeTagName);
						for (const attr of child.node.attributes) {
							newElement.setAttribute(attr.name, attr.value);
						}
						for (const child of child.node.childNodes) {
							newElement.appendChild(child);
						}
						
						child.node.replaceWith(newElement)
					});
				} else if (action.RemoveAttribute) {
					actions.push(() => {
						console.log("[HOT RELOAD] > RemoveAttribute", child.node, action.RemoveAttribute);
						child.node.removeAttribute(action.RemoveAttribute);
					});
				} else if (action.SetAttribute) {
					const [name, value] = action.SetAttribute;
					actions.push(() => {
						console.log("[HOT RELOAD] > SetAttribute", child.node, action.SetAttribute);
						child.node.setAttribute(name, value);
					});
				} else if (action.SetText) {
					const node = child.node;
					actions.push(() => {
						console.log("[HOT RELOAD] > SetText", child.node, action.SetText);
						node.textContent = action.SetText
					});
				} else if (action.AppendChildren) {
					actions.push(() => {
						console.log("[HOT RELOAD] > AppendChildren", child.node, action.AppendChildren);
						const newChildren = fromReplacementNode(action.AppendChildren, actualChildren);
						child.node.append(newChildren);
					});
				} else if (action.RemoveChild) {
					actions.push(() => {
						console.log("[HOT RELOAD] > RemoveChild", child.node, child.children, action.RemoveChild);
						const toRemove = child.children[action.RemoveChild.at];
						let toRemoveNode = toRemove.node;
						if (!toRemoveNode) {
							const range = new Range();
							range.setStartBefore(toRemove.start);
							range.setEndAfter(toRemove.end);
							toRemoveNode = range.deleteContents();
						} else {
							toRemoveNode.parentNode.removeChild(toRemoveNode);
						}
					})
				} else if (action.InsertChild) {
					const newChild = fromReplacementNode(action.InsertChild.child, actualChildren),
						before = child.children[action.InsertChild.before];
					actions.push(() => {
						console.log("[HOT RELOAD] > InsertChild", child, child.node, action.InsertChild, " before ", before);
						if (!before) {
							child.node.appendChild(newChild);
						} else {
							child.node.insertBefore(newChild, (before.node || before.start));
						}
					})
				} else if (action.InsertChildAfter) {
					const newChild = fromReplacementNode(action.InsertChildAfter.child, actualChildren),
						after = child.children[action.InsertChildAfter.after];
					actions.push(() => {
						console.log("[HOT RELOAD] > InsertChildAfter", child, child.node, action.InsertChildAfter, " after ", after);
						if (!after) {
							child.node.appendChild(newChild);
						} else {
							child.node.insertBefore(newChild, (after.node || after.start).nextSibling);
						}
					})
				} else {
					console.warn("[HOT RELOADING] Unmatched action", action);
				}
			}

			// actually run the actions
			// the reason we delay them is so that children aren't moved before other children are found, etc.
			for (const action of actions) {
				action();
			}
		}
	} catch (e) {
		console.warn("[HOT RELOADING] Error: ", e);
	}

	function fromReplacementNode(node, actualChildren) {
		if (node.Html) {
			return fromHTML(node.Html);
		} else {
			const child = childAtPath(
				actualChildren.length > 1 ? { children: actualChildren } : actualChildren[0],
				node.Path
			);
			if (child) {
				let childNode = child.node;
				if (!childNode) {
					const range = new Range();
					range.setStartBefore(child.start);
					range.setEndAfter(child.end);
					childNode = range.cloneContents();
				}
				return childNode;
			} else {
				console.warn("[HOT RELOADING] Could not find replacement node at ", node.Path);
				return undefined;
			}
		}
	}

		function buildActualChildren(element, range) {
		const walker = document.createTreeWalker(
			element,
			NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT | NodeFilter.SHOW_COMMENT,
			{
				acceptNode(node) {
					return node.parentNode == element && (!range || range.isPointInRange(node, 0))
				}
			}
		);
		const actualChildren = [],
			elementCount = {};
		while (walker.nextNode()) {
			if (walker.currentNode.nodeType == Node.ELEMENT_NODE) {
				if (elementCount[walker.currentNode.nodeName]) {
					elementCount[walker.currentNode.nodeName] += 1;
				} else {
					elementCount[walker.currentNode.nodeName] = 0;
				}
				elementCount[walker.currentNode.nodeName];

				actualChildren.push({
					type: "element",
					name: walker.currentNode.nodeName,
					number: elementCount[walker.currentNode.nodeName],
					node: walker.currentNode,
					children: buildActualChildren(walker.currentNode)
				});
			} else if (walker.currentNode.nodeType == Node.TEXT_NODE) {
				actualChildren.push({
					type: "text",
					node: walker.currentNode
				});
			} else if (walker.currentNode.nodeType == Node.COMMENT_NODE) {
				if (walker.currentNode.textContent.trim().startsWith("leptos-view")) {
				} else if (walker.currentNode.textContent.trim() == "<() />") {
					actualChildren.push({
						type: "unit",
						node: walker.currentNode
					});
				} else if (walker.currentNode.textContent.trim() == "<DynChild>") {
					let start = walker.currentNode;
					while (walker.currentNode.textContent.trim() !== "</DynChild>") {
						walker.nextNode();
					}
					let end = walker.currentNode;
					actualChildren.push({
						type: "dyn-child",
						start, end
					});
				} else if (walker.currentNode.textContent.trim().startsWith("<")) {
					let componentName = walker.currentNode.textContent.trim();
					let endMarker = componentName.replace("<", "</");
					let start = walker.currentNode;
					while (walker.currentNode.textContent.trim() !== endMarker) {
						walker.nextSibling();
					}
					let end = walker.currentNode;
					actualChildren.push({
						type: "component",
						start, end
					});
				}
			} else {
				console.warn("[HOT RELOADING] Building children, encountered", walker.currentNode);
			}
		}
		return actualChildren;
	}

	function childAtPath(element, path) {
		if (path.length == 0) {
			return element;
		} else if (element.children) {
			const next = element.children[path[0]],
				rest = path.slice(1);
			return childAtPath(next, rest);
		} else if (path == [0]) {
			return element;
		} else {
			console.warn("[HOT RELOADING] Child at ", path, "not found in ", element);
			return element;
		}
	}

	function fromHTML(html) {
		const template = document.createElement("template");
		template.innerHTML = html;
		return template.content.cloneNode(true);
	}
}
