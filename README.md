# Local SimilarArtists and TopSongs Navidrome Metadata Agent as Plugin
Navidrome plugin written in rust that provides local metadataAgent functionalities.
It will use the playcount/ratings/loves data from a navidrome-user of your choice and present you with _your_ Top Songs of an artist.
Doing it fully local has benefits (privacy, _your_ data), but also has limits. Check section [Limitations](#limitations) for more details.
You might still think this is favourable over having nothing if you don't want to - or cannot - enable any external services.

## Features
Locally resolves:
- Artist TopSongs
	- by weighting favorited, favourably rated and often-played songs higher. If no plays or ratings/favorite are available, it returns the artists newest songs
- Similar Artists
	- Checks for any collaborations with other artists. Artists that are collaborated with more often are considered more "similar"

These 2 are needed for "artist radio" etc. (getSimilarSongs)

## Usage

### 1. Download plugin file and copy into your pluginfolder
- Download the plugin file from [here](https://raw.githubusercontent.com/metalheim/navidrome_plugin_local_agent/refs/heads/main/plugin_local_agent.ndp)
- Copy the `plugin_local_agent.ndp` into your PluginFolder

> [!TIP]
> The default plugin folder is a folder called "plugins" in your datafolder

### 2. Enable plugins in Navidrome (if not already on):
_navidrome.toml:_
```toml
[Plugins]
	Enabled = true
```
### 3. Configure Agent
Add `plugin_local_agent` to your `Agents` configuration.
_navidrome.toml:_
```toml
Agents="plugin_local_agent,deezer,lastfm,spotify,local"
```

### 4. Configure the plugin
In Navidrome WebUI, go to <kbd>plugins</kbd> -> <kbd>plugin_local_agent</kbd>
1. **Enable Plugin**
2. **Users Permission**
  - Enable exactly 1 user
  - Alternatively "Allow all users", plugin will use the first admin user it sees
3. **Configuration**
> [!NOTE] 
> Settings in here are _optional_
  - `Skip Agent if no results` Set to `true` if you also use other metadataAgents
    - If no meaningful topSongs could be found by plugin, it will skip itself and the next metadataAgent is queried for topsongs.
    - Don't configure or set to `false` if you use this plugin as the _only_ metadata agent.

## Limitations

### Being single-user
Plugin uses Navidromes OpenSubsonic API to query the data needed for resolving TopSongs and SimilarArtists.
This API always needs a `user`. The data for TopSongs takes into consideration user annotations (favorites, playcount, ratings) and is therefore different for each user.
The plugin does not know which user queried for TopSongs, and it can only return TopSongs for one user.
#### example
`user A` wants to get TopSongs for `ArtistX` and this user has a several old songs of this artist favorited.
`user A` has all *new* songs of this artist rated 1-star, because he only likes `ArtistX`s old songs.

The plugin has `user B` configured as its data-source. `user B` has _only_ likes `ArtistX`s new songs.
Therefore the plugin will return `ArtistX`s **new** songs. Which `user A` doesn't like.

### General shortcomings
`BandA` might make very similar music to `BandB`. 
But they have never collaborated and never appear on the same album/sampler/whatever.
The only thing that makes the plugin consider them "similar" is if they have the same genres tagged in their music.

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