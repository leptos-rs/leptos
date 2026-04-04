(function (root, pkg_path, output_name, wasm_output_name) {
	import(`${root}/${pkg_path}/${output_name}.js`)
		.then(mod => {
			mod.default({module_or_path: `${root}/${pkg_path}/${wasm_output_name}.wasm`}).then(() => {
				mod.hydrate();
			});
		})
})
