(function (root, pkg_path, output_name, wasm_output_name, defer) {
	const pkg_url = pkg_path ? `${root}/${pkg_path}` : root;
	const hydrate = () =>
		import(`${pkg_url}/${output_name}.js`)
			.then(mod => {
				mod.default({module_or_path: `${pkg_url}/${wasm_output_name}.wasm`}).then(() => {
					mod.hydrate();
				});
			});
	if (defer && "requestIdleCallback" in window) {
		window.requestIdleCallback(hydrate, { timeout: 2000 });
	} else {
		hydrate();
	}
})
