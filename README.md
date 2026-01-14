# Local SimilarArtists and TopSongs Navidrome Metadata Agent as Plugin
Navidrome plugin written in rust that provides local metadataAgent functionalities.
Doing it fully local has limits, but might still be favourable over having nothing if you don't want to - or cannot - enable any external services.
See section #limitations for more details.

## Features
Locally resolves:
- Artist TopSongs
	- by weighting favorited, favourably rated and often-played songs higher. If no plays or ratings/favorite are available, it returns the artists newest songs
- Similar Artists
	- Checks for any collaborations with other artists. Artists that are collaborated with more often are considered more "similar"

These 2 are needed for "artist radio" etc. (getSimilarSongs)

## Configuration

### 1. copy plugin file into your plugifolder
Copy the `plugin_local_agent.ndp` into your PluginFolder
Your plugin folder is a folder called "plugins" in your datafolder by default.

### 2. Enable plugins in Navidrome (if not already on):
navidrome.toml:
```toml
[Plugins]
	Enabled = true
	CacheSize = "200MB"           # Compilation cache size limit
```
### 3. Configure Agent
navidrome.toml:
```toml
Agents="plugin_local_agent,deezer,lastfm,spotify,local"
```
### 4. Configure the plugin
1. In Navidrome WebUI, go to plugins -> plugin_local_agent
2. Plugin itself needs `user` permissions. 
  Enable at least 1 user. 
  If you "Enable access to all users", plugin will use the first admin user it sees.
3. (optional) set plugin-specific configuration
  - `user` (optional) configure a specific username that the plugin should use. 
  - `agent_skippable` (optional, default false). Set to `true` if you also use other metadataAgents. If no meaningful topSongs could be found by plugin, it will skip itself and the next metadataAgent is queried for topsongs.
  Don't configure or set to `false` if you use this plugin as the _only_ metadata agent.

## Limitations

### User behaviour
Plugin uses Navidromes OpenSubsonic API to query the data needed for resolving TopSongs and SimilarArtists.
This API always needs a `user`. The data for TopSongs takes into consideration user annotations (favorites, playcount, ratings) and is therefore different.
The plugin does not know which user queried for TopSongs, and it can only return TopSongs for one user.
#### example
`user A` wants to get TopSongs for `ArtistX` and this user has a few songs of this artist favorited.
`user A` has all *new* songs of this artist rated 1-star, because he only `ArtistX`s old music.

The plugin has `user B` configured as its data-source. `user B` has not listended to a single song of `ArtistX`

### General shortcomings
BandA might make very similar music to BandB. But they have never collaborated and never appear on the same album/sampler/whatever.
The only thing that makes the plugin consider them "similar" is if they use the same genres.


## Build the plugin yourself

### Requirements

- Rust toolchain with wasm32-wasip1 target
```bash
# Install the WASM target if you haven't already
rustup target add wasm32-wasip1
```
- Navidrome with plugins enabled

### Building

```bash
git clone https://github.com/metalheim/plugin_local_agent
cd plugin_local_agent
# Build the plugin (this calls cargo build and packages the wasm file+manifest into an ndp file
./build-ndp.sh
```