wasm-pack build --target=web --features=hydrate --release
cd pkg
rm *.br
cp hackernews.js hackernews.unmin.js
cat hackernews.unmin.js | esbuild > hackernews.js
brotli hackernews.js
brotli hackernews_bg.wasm
brotli style.css