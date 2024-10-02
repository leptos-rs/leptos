((root, pkg_path, output_name, wasm_output_name) => {
	function idle(c) {
		if ("requestIdleCallback" in window) {
			window.requestIdleCallback(c);
		} else {
			c();
		}
	}
	function islandTree(rootNode) {
		const tree = [];

		function traverse(node, parent) {
			if (node.nodeType === Node.ELEMENT_NODE) {
				if(node.tagName.toLowerCase() === 'leptos-island') {
					const children = [];
					const id = node.dataset.component || null;
					const data = { id, node, children };
					
					for(const child of node.children) {
						traverse(child, children);
					}

					(parent || tree).push(data);
				} else {
					for(const child of node.children) {
						traverse(child, parent);
					};
				}
			}
		}

		traverse(rootNode, null);

		return { el: null, id: null, children: tree };
	}
	function hydrateIsland(el, id, mod) {
		const islandFn = mod[id];
		if (islandFn) {
			islandFn(el);
		} else {
			console.warn(`Could not find WASM function for the island ${id}.`);
		}
	}
	function hydrateIslands(entry, mod) {
		if(entry.node) {
			hydrateIsland(entry.node, entry.id, mod);
		}
		for (const island of entry.children) {
			hydrateIslands(island, mod);
		}
	}
	idle(() => {
		import(`${root}/${pkg_path}/${output_name}.js`)
			.then(mod => {
				mod.default(`${root}/${pkg_path}/${wasm_output_name}.wasm`).then(() => {
					mod.hydrate();
					hydrateIslands(islandTree(document.body, null), mod);
				});
			})
	});
})
