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

			/*    
				ReplaceWith(LNode),
				ChangeTagName(String),
				RemoveAttribute(String),
				SetAttribute(String, String),
				SetText(String),
				ClearChildren,
				AppendChildren(Vec<LNode>),
				AppendChild { at: usize, child: LNode },
				RemoveChild { at: usize },
				InsertChild { before: usize, child: LNode },
				MoveChild { from: usize, to: usize },
			*/

			// build up the set of actions
			for (const patch of patches) {
				const child = childAtPath(
					actualChildren.length > 1 ? { children: actualChildren } : actualChildren[0],
					patch.path
				);
				const action = patch.action;
				if (action == "ClearChildren") {
					console.log("[HOT RELOAD] > ClearChildren", child.node);
					actions.push(() => child.node.textContent = "");
				} else if (action.ReplaceWith) {
					console.log("[HOT RELOAD] > ReplaceWith", child.node, action.ReplaceWith);
					actions.push(() => child.node.replaceWith(fromHTML(actions.ReplaceWith)));
				} else if (action.ChangeTagName) {
					console.log("[HOT RELOAD] > ChangeTagName", child.node, action.ChangeTagName);
					actions.push(() => {
						const newElement = document.createElement(action.ChangeTagName);
						for (const attr of child.node.attributes) {
							newElement.setAttribute(attr.name, attr.value);
						}
						for (const child of child.node.childNodes) {
							newElement.appendChild(child);
						}
						
						child.node.replaceWith(newElement)
					});
				} else if (action.SetAttribute) {
					console.log("[HOT RELOAD] > SetAttribute", child.node, action.SetAttribute);
					const [name, value] = action.SetAttribute;
					actions.push(() => {
						child.node.setAttribute(name, value);
					});
				} else if (action.SetText) {
					console.log("[HOT RELOAD] > SetText", child.node, action.SetText);
					actions.push(() => child.node.textContent = action.SetText);
				} else if (action.AppendChildren) {
					console.log("[HOT RELOAD] > AppendChildren", child.node, action.AppendChildren);
					actions.push(() => {
						const newChildren = fromHTML(action.AppendChildren);
						child.node.append(newChildren);
					});
				} else if (action.RemoveChild) {
					console.log("[HOT RELOAD] > RemoveChild", child.node, child.children, action.RemoveChild);
					actions.push(() => {
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
					console.log("[HOT RELOAD] > InsertChild", child.node, action.InsertChild);
					const newChild = fromHTML(action.InsertChild.child),
						before = child.children[action.InsertChild.at];
					actions.push(() => {
						if (!before) {
							child.node.appendChild(newChild);
						} else {
							child.node.insertBefore(newChild, (before.node || before.start));
						}
					})
				} else if (action.MoveChild) {
					console.log("[HOT RELOAD] > MoveChild", child, action.MoveChild);
					const fromEl = child.children[action.MoveChild.from],
						nextToEl = child.children[action.MoveChild.to + 1];
					let fromNode = fromEl.node;
					if (!fromNode) {
						const range = new Range();
						range.setStartBefore(fromEl.start);
						range.setEndAfter(fromEl.end);

						// keep the range in the DOM until we've been able to move
						// anything else around it, then delete it
						fromNode = range.cloneContents();
						requestAnimationFrame(() => {
							const range = new Range();
							range.setStartBefore(fromEl.start);
							range.setEndAfter(fromEl.end);
							range.deleteContents();
						});
					}
					actions.push(() => {
						if (nextToEl) {
							console.log("insertBefore", nextToEl.node || nextToEl.start);
							child.node.insertBefore(fromNode, nextToEl.node || nextToEl.start);
						} else {
							child.node.appendChild(fromNode);
						}
					});
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
		} else {
			const next = element.children[path[0]],
				rest = path.slice(1);
			return childAtPath(next, rest);
		}
	}

	function fromHTML(html) {
		const template = document.createElement("template");
		template.innerHTML = html;
		return template.content.cloneNode(true);
	}
}
