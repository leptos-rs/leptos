(function (root, pkg_path, output_name, wasm_output_name, defer) {
	const hydrate = () =>
		import(`${root}/${pkg_path}/${output_name}.js`)
			.then(mod => {
				mod.default({module_or_path: `${root}/${pkg_path}/${wasm_output_name}.wasm`}).then(() => {
					mod.hydrate();
				});
			});
	if (defer && "requestIdleCallback" in window) {
		// `timeout` is a backstop: a never-idle page would otherwise stay dead
		window.requestIdleCallback(hydrate, { timeout: 2000 });
	} else {
		hydrate();
	}
})
