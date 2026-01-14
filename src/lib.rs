use extism_pdk::*;
use nd_pdk::host::{subsonicapi, config, users};
use nd_pdk::metadata::{
    SimilarArtistsProvider, SimilarArtistsRequest, SimilarArtistsResponse, ArtistRef, ArtistTopSongsProvider, TopSongsRequest, TopSongsResponse, SongRef, Error
};
use serde::Serialize;
use std::collections::HashMap;

nd_pdk::register_metadata_similar_artists!(LocalProvider);
nd_pdk::register_metadata_artist_top_songs!(LocalProvider);
//TODO: resolve artist biographies locally (from text file or from artist.nfo)
#[derive(Default)]
struct LocalProvider;

impl SimilarArtistsProvider for LocalProvider {
	fn get_similar_artists(&self, req: SimilarArtistsRequest) -> Result<SimilarArtistsResponse, Error> {
		let artist_id 	= &req.id;
        let artist_name = &req.name;
        let limit 		= &req.limit;
		let username = match get_user_name() {
			Some(u) => u,
			None => return Err(Error::new("Could not get user name in get_similar_artists()")),
		};
		
		#[derive(Serialize, Debug, Clone)]
		struct SimilarArtist {
			id: String,
			name: String,
			relevance: u32,
		}
		let mut similar_artists: HashMap<String, SimilarArtist> = HashMap::new();
		let mut artist_genres: HashMap<String, u32> = HashMap::new();
		
		//weighting for different 
		const WEIGHT_ARTISTS_COLLABORATED_ON_ALBUM	:u32	= 100;	//will be multiplied by how many shared albums (both are albumuartist) artist has
		const WEIGHT_ARTISTS_ARE_IN_COMPILATION		:u32	= 10;	//will be multiplied by artist occurence in shared compilations which could be a lot (f.e. artist appears in 20 compilations and in each of these another artist also appears, score will be 20*baseweight)
		const WEIGHT_ARTISTS_SHARE_SAME_GENRE 		:u32	= 1;	//will be multiplied by genre weight AND albumartist-occurence which could be a lot (f.e. if the inputartist has 20 "hip-hop" albums and another artist also has 20 "hip-hop" albums, then the score could be 400*baseweight
		
		info!("Trying to locally resolve Similar Artists to: {}", artist_name);
		
		let url = format!("getArtist?u={}&id={}",username,artist_id);
		
		debug!("  Calling {}", url);
		
		let subsonic_response  = match subsonicapi::call(&url) {
			Ok(resp) => resp,
			Err(e) => {
				error!("  Failed to get albums for artist: {}", e);
				return Err(Error::new(format!("  Failed to get albums for artist: {}", e)));
			}
		};
		
		// Parse the response as JSON
		let subsonic_json: serde_json::Value = match serde_json::from_str(&subsonic_response) {
			Ok(json) => json,
			Err(e) => {
				error!("  Failed to parse JSON: {}", e);
				return Err(Error::new(format!("  Failed to parse JSON: {}", e)));
			}
		};
		
		let albums = subsonic_json["subsonic-response"]["artist"]["album"].as_array()
            .ok_or_else(|| Error::new("  No child array in response"))?;
		
		debug!("  Fetched all albums for: {} - Count: {}", artist_name, albums.len());
		
		for album in albums {
            if let Some(artists) = album["artists"].as_array() {
                for artist in artists {
					let id   = artist["id"].as_str().unwrap_or("").to_string();
					let name = artist["name"].as_str().unwrap_or("").to_string();
					if id != *artist_id { //artist should not be similar to itself
						let entry = similar_artists.entry(artist_id.clone()).or_insert(
							SimilarArtist { id: artist_id.clone(), name: name.clone(), relevance: 0 }
						);
						entry.relevance += WEIGHT_ARTISTS_COLLABORATED_ON_ALBUM;
					}
                }
            }
			//Step 2: Check compilations and add artists on the same compilation as "similar"
			let is_compilation = album.get("isCompilation")
				.and_then(|v| v.as_bool())
				.unwrap_or(false);
			if is_compilation {
				let album_id = album["id"].as_str().unwrap_or("").to_owned();
				let album_url = format!("getAlbum.view?u={}&id={}", username, album_id);
				
				let album_response = match subsonicapi::call(&album_url) {
					Ok(resp) => resp,
					Err(_) => continue,
				};
				let album_json: serde_json::Value = match serde_json::from_str(&album_response) {
					Ok(json) => json,
					Err(_) => continue,
				};

				if let Some(songs_array) = album_json["subsonic-response"]["album"]["song"].as_array() {
					for song in songs_array {
						if let Some(artists) = song["artists"].as_array() {
							for artist in artists {
								let name = artist["name"].as_str().unwrap_or("").to_string();
								let id   = artist["id"].as_str().unwrap_or("").to_string();
								if id != *artist_id { //artist should not be similar to itself
									let entry = similar_artists.entry(artist_id.clone()).or_insert(
										SimilarArtist { id: artist_id.clone(), name: name.clone(), relevance: WEIGHT_ARTISTS_ARE_IN_COMPILATION }
									);
									entry.relevance += WEIGHT_ARTISTS_ARE_IN_COMPILATION;
								}
							}
						}
					}
				}	
			}
			//Save all artists genres to another hasmap used for step 3.
			if let Some(genres) = album["genres"].as_array() {
                for genre in genres {
					let genrename = genre["name"].as_str().unwrap_or("").to_string();
						let relevance = artist_genres.entry(genrename.clone()).or_insert(1);
						*relevance += 1;
                }
            }
		}
		
		// Step 3: Also check for similar artists by getting albums with the same genres using getalbumlist2 filtered by genre
		// iterate through artist_genres and fetch albums foreach, then take artists from these albums
        let mut top_genres: Vec<(String, u32)> = artist_genres.into_iter().collect();
        top_genres.sort_by(|a, b| b.1.cmp(&a.1));
        top_genres.truncate(5 as usize); //TODO: dont hardcode genre limit of 5
		
		for (genre_name, genre_weight) in &top_genres {
			match genre_name.as_str() {
				_ => {
					let url = format!("getAlbumList2?u={}&type=byGenre&genre={}",username,genre_name);
					
					debug!("  Calling {}", url);
					let subsonic_response  = match subsonicapi::call(&url) {
						Ok(resp) => resp,
						Err(e) => {
							error!("  Failed to get albums for genre {}", e);
							return Err(Error::new(format!("  Failed to get albums for genre {}", e)));
						}
					};
					
					// Parse the response as JSON
					let subsonic_json: serde_json::Value = match serde_json::from_str(&subsonic_response) {
						Ok(json) => json,
						Err(e) => {
							error!("  Failed to parse JSON: {}", e);
							return Err(Error::new(format!("  Failed to parse JSON: {}", e)));
						}
					};
					
					let albums = subsonic_json["subsonic-response"]["albumList2"]["album"].as_array()
						.ok_or_else(|| Error::new("  No album array in response"))?;
					
					for album in albums {
						if let Some(artists) = album["artists"].as_array() {
							for artist in artists {
								let name = artist["name"].as_str().unwrap_or("").to_string();
								let id   = artist["id"].as_str().unwrap_or("").to_string();
								
								if id != *artist_id { //artist should not be similar to itself
									let entry = similar_artists.entry(artist_id.clone()).or_insert(
										SimilarArtist { id: artist_id.clone(), name: name.clone(), relevance: WEIGHT_ARTISTS_ARE_IN_COMPILATION }
									);
									entry.relevance += WEIGHT_ARTISTS_SHARE_SAME_GENRE * genre_weight;
								}
							}
						}
					}
		
				}
			}
		}
		
		// convert hasmap to array, sort descending and take top `limit`
        let mut entries: Vec<(String, SimilarArtist)> = similar_artists.into_iter().collect();
        entries.sort_by(|a, b| b.1.relevance.cmp(&a.1.relevance));
        entries.truncate(*limit as usize);
		
		// Serialize entries to JSON and print/log
		match serde_json::to_string(&entries) {
			Ok(json) => {
				debug!("  Top similar artists: {}", json);
			},
			Err(e) => {
				error!("  Could not serialize entries to JSON: {}", e);
			}
		}
		
		 // Map to ArtistRef
        let artists: Vec<ArtistRef> = entries.into_iter()
            .map(|(_id, artist)| ArtistRef {
				id: artist.id,
				name: artist.name,
				mbid: "".to_string(),
			})
			.collect();

        Ok(SimilarArtistsResponse { artists })
	}
}

impl ArtistTopSongsProvider for LocalProvider {
    fn get_artist_top_songs(&self, req: TopSongsRequest) -> Result<TopSongsResponse, Error> {
        let artist_id   = &req.id;
        let artist_name = &req.name;
        let limit       = req.count;
        info!("Trying to locally resolve Top Songs for: {}", artist_name);
		let username = match get_user_name() {
			Some(u) => u,
			None => return Err(Error::new("  Could not get user name in get_artist_top_songs()")),
		};
		
		let config_agent_skippable = match config::get("agent_skippable") {
			Ok((value, _is_set)) => {
				// If the value is string "true" or "false", parse it:
				value.parse::<bool>().unwrap_or(false)
			}
			Err(_) => false,
		};

        // 1st API call: get artist directory (albums)
        let url = format!("getMusicDirectory?u={}&id={}",username,artist_id);
        let artist_response = match subsonicapi::call(&url) {
            Ok(resp) => resp,
            Err(e) => return Err(Error::new(format!("  Failed to get albums for artist: {}", e))),
        };
        let artist_json: serde_json::Value = match serde_json::from_str(&artist_response) {
            Ok(json) => json,
            Err(e) => {
                error!("  Failed to parse JSON: {}", e);
                return Err(Error::new(format!("  Failed to parse JSON: {}", e)));
            }
        };
        let albums = artist_json["subsonic-response"]["directory"]["child"]
            .as_array()
            .ok_or_else(|| Error::new("  No album array in response"))?;

        let mut topsongs: Vec<(String, String, f32, u16, String)> = Vec::new();

        // 2. For each album, get its tracks
        for album in albums {
			if !album["isDir"].as_bool().unwrap_or(false) {
				continue; // skip non-albums
			}
			let album_id = match album["id"].as_str() {
				Some(id) => id,
				None => continue,
			};

			let album_url = format!("getAlbum.view?u={}&id={}", username, album_id);
			let album_response = match subsonicapi::call(&album_url) {
				Ok(resp) => resp,
				Err(_) => continue, // skip on failure
			};
			let album_json: serde_json::Value = match serde_json::from_str(&album_response) {
				Ok(json) => json,
				Err(_) => continue, // skip on failure
			};

			if let Some(songs_array) = album_json["subsonic-response"]["album"]["song"].as_array() {
				for song in songs_array {
					let song_name 	= song["title"].as_str().unwrap_or("").to_owned();
					let song_id 	= song["id"].as_str().unwrap_or("").to_owned();
					if song_id.is_empty() {
						continue;
					}
					let play_count = song["playCount"].as_i64().unwrap_or(0) as f32;
					let rating = song["userRating"].as_f64().unwrap_or(0.0) as f32;
					let loved = song["starred"].is_string();
					let year = song["year"].as_i64().unwrap_or(0) as u16;
					let musicbrainz_id = song["musicBrainzId"].as_str().unwrap_or("").to_owned();
					let main_artist_name = artist_name;
					// Check if main artist is among performing artists
					let mut is_artist_in_song_artists = false;
					if let Some(song_artists) = song["artists"].as_array() {
						for a in song_artists {
							let name = a["name"].as_str().unwrap_or("");
							if name.to_lowercase() == main_artist_name.to_lowercase() {
								is_artist_in_song_artists = true;
								break;
							}
						}
					}
					
					let mut is_artist_contributor = false;
					if let Some(contributors) = song["contributors"].as_array() {
						for c in contributors {
							if let Some(artist_obj) = c.get("artist") {
								let name = artist_obj["name"].as_str().unwrap_or("");
								if name.to_lowercase() == main_artist_name.to_lowercase() {
									is_artist_contributor = true;
									break;
								}
							}
						}
					}
					
					// === Weight calculation ===
					let weight = if loved { 5.0 } else { rating };
					let effective_weight = if weight != 0.0 {
						(weight - 2.5) * (play_count + 1.0)
					} else {
						play_count + 1.0
					};

					// Apply decrease if artist is NOT in song-artists
					let mut final_weight = effective_weight;
					
					// slightly decrease weight if artist is NOT in song-artists but is a contributor (producer etc.)
					if !is_artist_in_song_artists && is_artist_contributor {
						final_weight = final_weight/2.0;
					}
					
					if is_artist_in_song_artists || is_artist_contributor {
						topsongs.push((song_id, song_name, final_weight, year, musicbrainz_id));
					}
				}
			}
		}

		info!("  {} has {} songs from subsonic api, sorting by weight and limiting amount to {} ", artist_name, topsongs.len(), limit);
		
		// Sort by weight DESC, then year DESC, then limit amount
		topsongs.sort_by(|a, b| {
			match b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal) {
				std::cmp::Ordering::Equal => b.3.cmp(&a.3),
				ord => ord,
			}
		});
		topsongs.truncate(limit as usize);
		
		//If no meaningful data could be gathered, error out. Navidrome will try the next agent (if any configured)
		if topsongs.is_empty() || (	topsongs.iter().all(|t| t.2 <= 1.0)	&& config_agent_skippable) {
			error!("  No meaningful sorting for songs fround, skipping agent");
			return Err(Error::new("No meaningful top songs found (none above weight threshold)"));
		}
        match serde_json::to_string(&topsongs) {
            Ok(json) => info!("  Top songs: {}", json),
            Err(e) => error!("  Could not serialize entries to JSON: {}", e),
        }

		// Mapping
		let song_refs: Vec<SongRef> = topsongs.into_iter()
			.map(|(song_id, song_name, _weight, _year, musicbrainz_id)| SongRef {
				id:   song_id,
				name: song_name,
				mbid: musicbrainz_id,
			})
			.collect();
		
        info!("  Successfully resolved topsong for: {}, found {} songs", artist_name, song_refs.len());
        Ok(TopSongsResponse { songs: song_refs })
    }
}

// =============================== 
// HELPER FUNCTIONS
// =============================== 

fn get_user_name() -> Option<String> {
    // 1. Try config username
    if let Ok((config_username, true)) = config::get("username") {
        // Check if this username exists among users
        if let Ok(available_users) = nd_pdk::host::users::get_users() {
            if available_users.iter().any(|u| u.user_name.to_lowercase() == config_username.to_lowercase()) {
                return Some(config_username);
            }
        }
    }

    // 2. Fallback: first admin user
    if let Ok(admins) = users::get_admins() {
        if let Some(firstadmin) = admins.into_iter().next() {
            return Some(firstadmin.user_name);
        }
    }

    // 3. No user found (shouldn't happen as at least 1 admin user is required)
    error!("Couldn't find any user from configuration");
    None
}
