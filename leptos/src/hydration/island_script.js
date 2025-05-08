((root, pkg_path, output_name, wasm_output_name) => {
	let MOST_RECENT_CHILDREN_CB = [];

	function idle(c) {
		if ("requestIdleCallback" in window) {
			window.requestIdleCallback(c);
		} else {
			c();
		}
	}
	function hydrateIslands(rootNode, mod) {
		function traverse(node) {
			if (node.nodeType === Node.ELEMENT_NODE) {
				const tag = node.tagName.toLowerCase();
				if(tag === 'leptos-island') {
					const children = [];
					const id = node.dataset.component || null;

					hydrateIsland(node, id, mod);
					
					for(const child of node.children) {
						traverse(child, children);
					}
				} else {
					if (tag === 'leptos-children') {
						MOST_RECENT_CHILDREN_CB.push(node.$$on_hydrate);
						for(const child of node.children) {
							traverse(child);
						};
						// un-set the "most recent children"
						MOST_RECENT_CHILDREN_CB.pop();
					} else {
						for(const child of node.children) {
							traverse(child);
						};
					}
				}
			}
		}

		traverse(rootNode);
	}
	function hydrateIsland(el, id, mod) {
		const islandFn = mod[id];
		if (islandFn) {
			const children_cb = MOST_RECENT_CHILDREN_CB[MOST_RECENT_CHILDREN_CB.length-1];
			if (children_cb) {
				children_cb();
			}
			islandFn(el);
		} else {
			console.warn(`Could not find WASM function for the island ${id}.`);
		}
	}
	idle(() => {
		import(`${root}/${pkg_path}/${output_name}.js`)
			.then(mod => {
				mod.default({module_or_path: `${root}/${pkg_path}/${wasm_output_name}.wasm`}).then(() => {
					mod.hydrate();
					hydrateIslands(document.body, mod);
				});

				window.__hydrateIsland = (el, id) => hydrateIsland(el, id, mod);
			})
	});
})
