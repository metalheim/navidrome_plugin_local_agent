cargo build --release --target wasm32-wasip1
cp -f ./target/wasm32-wasip1/release/plugin_local_agent.wasm ./plugin.wasm
zip -j plugin_local_agent.ndp manifest.json plugin.wasm
rm -f ./plugin.wasm
cp -f plugin_local_agent.ndp ../navidrome/plugins