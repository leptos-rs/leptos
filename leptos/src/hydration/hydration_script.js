(function (root_prefix, pkg_path, output_name, wasm_output_name) {
	import(`${root_prefix}${pkg_path}/${output_name}.js`)
		.then(mod => {
			mod.default({module_or_path: `${root_prefix}${pkg_path}/${wasm_output_name}.wasm`}).then(() => {
				mod.hydrate();
			});
		})
})
