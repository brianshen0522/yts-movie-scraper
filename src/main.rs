use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const API_BASE: &str = "https://yts.bz/api/v2/list_movies.json";
const OUTPUT_FILE: &str = "yts_movies.json";
const LIMIT: u32 = 50;

#[derive(Parser)]
#[command(name = "YTS Grabber")]
#[command(about = "A toolkit for managing YTS movie database", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch movies from YTS (default: only new movies after first run)
    Fetch,
    /// List all movies in the database
    List {
        /// Show only first N movies
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Check how many new movies are available without downloading
    Check,
    /// Calculate total size of the largest torrent from each movie
    Size,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Torrent {
    quality: String,
    hash: String,
    magnet_url: String,
    size_bytes: u64,
    size: String, // Human readable size like "1.84 GB"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Movie {
    id: u32,
    title: String,
    year: u32,
    imdb_code: String,
    torrents: Vec<Torrent>,
}

#[derive(Debug, Deserialize)]
struct ApiMovie {
    id: u32,
    title: String,
    year: u32,
    imdb_code: String,
    torrents: Vec<ApiTorrent>,
}

#[derive(Debug, Deserialize)]
struct ApiTorrent {
    quality: String,
    #[serde(rename = "type")]
    torrent_type: String,
    hash: String,
    size: String,
    size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct ApiData {
    movie_count: u32,
    movies: Option<Vec<ApiMovie>>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: ApiData,
}

fn create_magnet_url(hash: &str, title: &str) -> String {
    let encoded_title = title.replace(' ', "+");
    format!(
        "magnet:?xt=urn:btih:{}&dn={}&tr=udp://open.demonii.com:1337/announce&tr=udp://tracker.openbittorrent.com:80&tr=udp://tracker.coppersurfer.tk:6969&tr=udp://glotorrents.pw:6969/announce&tr=udp://tracker.opentrackr.org:1337/announce&tr=udp://torrent.gresille.org:80/announce&tr=udp://p4p.arenabg.com:1337&tr=udp://tracker.leechers-paradise.org:6969",
        hash, encoded_title
    )
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn fetch_page(page: u32) -> Result<ApiResponse> {
    let url = format!("{}?limit={}&page={}&sort_by=date_added&order_by=desc", 
                     API_BASE, LIMIT, page);
    
    let response = reqwest::blocking::get(&url)?
        .json::<ApiResponse>()?;
    
    Ok(response)
}

fn load_existing_movies() -> Result<Vec<Movie>> {
    if !Path::new(OUTPUT_FILE).exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(OUTPUT_FILE)?;
    let movies: Vec<Movie> = serde_json::from_str(&content)?;
    Ok(movies)
}

fn save_movies(movies: &[Movie]) -> Result<()> {
    let json = serde_json::to_string_pretty(movies)?;
    fs::write(OUTPUT_FILE, json)?;
    Ok(())
}

fn check_new_movies() -> Result<()> {
    println!("🔍 Checking for new movies...\n");
    
    let existing_movies = load_existing_movies()?;
    let latest_id = existing_movies.iter().map(|m| m.id).max().unwrap_or(0);
    
    let first_response = fetch_page(1)?;
    let total_count = first_response.data.movie_count;
    
    println!("📊 Total movies on YTS: {}", total_count);
    
    if latest_id == 0 {
        println!("📁 No local database found");
        println!("🆕 All {} movies are new", total_count);
        return Ok(());
    }
    
    println!("📁 Local database has {} movies", existing_movies.len());
    println!("🔍 Latest movie ID in database: {}\n", latest_id);
    
    let mut new_movie_count = 0;
    let mut page = 1;
    let mut found_existing = false;
    
    print!("🔎 Scanning for new movies...");
    std::io::Write::flush(&mut std::io::stdout())?;
    
    loop {
        let response = fetch_page(page)?;
        if let Some(movies) = response.data.movies {
            for movie in movies {
                if movie.id <= latest_id {
                    found_existing = true;
                    break;
                }
                new_movie_count += 1;
            }
            if found_existing {
                break;
            }
            page += 1;
        } else {
            break;
        }
    }
    
    println!(" Done!\n");
    
    if new_movie_count == 0 {
        println!("✅ Database is up to date! No new movies available.");
    } else {
        println!("🆕 Found {} new movies available!", new_movie_count);
        println!("💡 Run 'fetch' command to download them.");
    }
    
    Ok(())
}

fn list_movies(limit: Option<usize>) -> Result<()> {
    let movies = load_existing_movies()?;
    
    if movies.is_empty() {
        println!("📁 No movies in database. Run 'fetch' command first.");
        return Ok(());
    }
    
    let display_count = limit.unwrap_or(movies.len()).min(movies.len());
    
    println!("🎬 Movies in Database: {} total\n", movies.len());
    println!("{:-<100}", "");
    
    for (idx, movie) in movies.iter().take(display_count).enumerate() {
        println!("{}. [ID: {}] {} ({})", 
                 idx + 1, 
                 movie.id, 
                 movie.title, 
                 movie.year);
        println!("   IMDb: {}", movie.imdb_code);
        println!("   Torrents: {}", movie.torrents.len());
        
        for torrent in &movie.torrents {
            println!("     - {} | {} | {}", 
                     torrent.quality, 
                     torrent.size,
                     torrent.hash);
        }
        println!("{:-<100}", "");
    }
    
    if display_count < movies.len() {
        println!("\n... and {} more movies", movies.len() - display_count);
        println!("💡 Use --limit to show more movies");
    }
    
    Ok(())
}

fn calculate_size() -> Result<()> {
    let movies = load_existing_movies()?;
    
    if movies.is_empty() {
        println!("📁 No movies in database. Run 'fetch' command first.");
        return Ok(());
    }
    
    println!("📊 Calculating total size...\n");
    
    let mut total_bytes: u64 = 0;
    let mut movies_with_torrents = 0;
    
    for movie in &movies {
        if let Some(largest_torrent) = movie.torrents.iter().max_by_key(|t| t.size_bytes) {
            total_bytes += largest_torrent.size_bytes;
            movies_with_torrents += 1;
        }
    }
    
    println!("🎬 Total movies: {}", movies.len());
    println!("📦 Movies with torrents: {}", movies_with_torrents);
    println!("💾 Combined size (largest torrent per movie): {}", format_size(total_bytes));
    
    if movies_with_torrents > 0 {
        println!(
            "📈 Average size per movie: {}",
            format_size(total_bytes / movies_with_torrents as u64)
        );
    } else {
        println!("📈 Average size per movie: N/A (no movies with torrents)");
    }
    
    Ok(())
}

fn fetch_movies() -> Result<()> {
    println!("🎬 YTS Movie Grabber Starting...\n");
    
    let existing_movies = load_existing_movies()?;
    let latest_id = existing_movies.iter().map(|m| m.id).max().unwrap_or(0);
    
    println!("📊 Fetching movie count...");
    let first_response = fetch_page(1)?;
    let total_count = first_response.data.movie_count;
    
    println!("Total movies in YTS: {}\n", total_count);
    
    if latest_id > 0 {
        println!("📁 Found existing database with {} movies", existing_movies.len());
        println!("🔍 Latest movie ID in database: {}\n", latest_id);
    }
    
    let mut all_new_movies: Vec<Movie> = Vec::new();
    let mut page = 1;
    let mut found_existing = false;
    
    let mut new_movie_count = 0;
    if latest_id > 0 {
        let mut temp_page = 1;
        loop {
            let response = fetch_page(temp_page)?;
            if let Some(movies) = response.data.movies {
                for movie in movies {
                    if movie.id <= latest_id {
                        found_existing = true;
                        break;
                    }
                    new_movie_count += 1;
                }
                if found_existing {
                    break;
                }
                temp_page += 1;
            } else {
                break;
            }
        }
        
        if new_movie_count == 0 {
    let pb = if latest_id > 0 {
        ProgressBar::new(new_movie_count as u64)
    } else {
        ProgressBar::new_spinner()
    };
        }
        
        println!("🆕 Found {} new movies to fetch\n", new_movie_count);
        found_existing = false;
    }
    
    let progress_total = if latest_id > 0 { new_movie_count } else { total_count };
    let pb = ProgressBar::new(progress_total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} movies ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    loop {
                    let quality_label = format!("{}-{}", t.quality, t.torrent_type);
                    
                    Torrent {
                        quality: quality_label,
                if api_movie.id <= latest_id {
                    found_existing = true;
                    break;
                }
                
                let torrents: Vec<Torrent> = api_movie.torrents.iter().map(|t| {
                    let magnet = create_magnet_url(&t.hash, &api_movie.title);
                    let quality_with_type = format!("{}-{}", t.quality, t.torrent_type);
                    
                    Torrent {
                        quality: quality_with_type,
                        hash: t.hash.clone(),
                        magnet_url: magnet,
                        size_bytes: t.size_bytes,
                        size: t.size.clone(),
                    }
                }).collect();
                
                let movie = Movie {
                    id: api_movie.id,
                    title: api_movie.title,
                    year: api_movie.year,
                    imdb_code: api_movie.imdb_code,
                    torrents,
                };
                
                all_new_movies.push(movie);
                pb.inc(1);
            }
            
            if found_existing {
                break;
            }
            
            page += 1;
        } else {
            break;
        }
    }
    
    pb.finish_with_message("✅ Fetching complete");
    
    println!("\n💾 Saving to {}...", OUTPUT_FILE);
    
    all_new_movies.extend(existing_movies);
    all_new_movies.sort_by(|a, b| b.id.cmp(&a.id));
    
    save_movies(&all_new_movies)?;
    
    println!("✅ Successfully saved {} total movies!", all_new_movies.len());
    println!("📝 File: {}", OUTPUT_FILE);
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Fetch) | None => fetch_movies()?,
        Some(Commands::List { limit }) => list_movies(limit)?,
        Some(Commands::Check) => check_new_movies()?,
        Some(Commands::Size) => calculate_size()?,
    }
    
    Ok(())
}