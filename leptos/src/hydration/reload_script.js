let host = window.location.hostname;
let ws = new WebSocket(`${protocol}${host}:${reload_port}/live_reload`);
ws.onmessage = (ev) => {
	let msg = JSON.parse(ev.data);
	if (msg.all) window.location.reload();
	if (msg.css) {
		let found = false;
		document.querySelectorAll("link").forEach((link) => {
			if (link.getAttribute('href').includes(msg.css)) {
				let newHref = '/' + msg.css + '?version=' + new Date().getMilliseconds();
				link.setAttribute('href', newHref);
				found = true;
			}
		});
		if (!found) console.warn(`CSS hot-reload: Could not find a <link href=/\"${msg.css}\"> element`);
	};
	if(msg.view) {
		patch(msg.view);
	}
};
ws.onclose = () => console.warn('Live-reload stopped. Manual reload necessary.');
