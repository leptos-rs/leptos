(function (root, pkg_path, output_name, wasm_output_name) {
	import(`${root}/${pkg_path}/${output_name}.js`)
		.then(mod => {
			mod.default({
				module_or_path: new Request(
					`${root}/${pkg_path}/${wasm_output_name}.wasm`,
					{ "credentials": "same-origin" }
				)
			}).then(() => {
				mod.hydrate();
			});
		})
})
