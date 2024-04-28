(function (pkg_path, output_name, wasm_output_name) {
	function idle(c) {
		if ("requestIdleCallback" in window) {
			window.requestIdleCallback(c);
		} else {
			c();
		}
	}
	idle(() => {
		import(`/${pkg_path}/${output_name}.js`)
			.then(mod => {
				mod.default(`/${pkg_path}/${wasm_output_name}.wasm`).then(() => {
					mod.hydrate();
					for (let e of document.querySelectorAll("leptos-island")) {
						const l = e.dataset.component;
						const islandFn = mod["_island_" + l];
						if (islandFn) {
							islandFn(e);
						} else {
							console.warn(`Could not find WASM function for the island ${l}.`);
						}
					}
				});
			})
	});
})
