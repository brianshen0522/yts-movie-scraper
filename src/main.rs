use anyhow::Result;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const API_BASE: &str = "https://yts.bz/api/v2/list_movies.json";
const OUTPUT_FILE: &str = "yts_movies.json";
const LIMIT: u32 = 50;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Torrent {
    quality: String,
    hash: String,
    size_bytes: u64,
    magnet_url: String,
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

#[derive(Parser)]
#[command(name = "YTS Movie Grabber")]
#[command(about = "A toolkit for managing YTS movie database", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch movies from YTS (default: fetch new movies only)
    Fetch,
    
    /// List all movies in the database
    List,
    
    /// Show statistics about the database
    Stats,
    
    /// Calculate total size of all torrents (uses biggest torrent per movie)
    Size,
}

fn create_magnet_url(hash: &str, title: &str) -> String {
    let encoded_title = title.replace(' ', "+");
    format!(
        "magnet:?xt=urn:btih:{}&dn={}&tr=udp://open.demonii.com:1337/announce&tr=udp://tracker.openbittorrent.com:80&tr=udp://tracker.coppersurfer.tk:6969&tr=udp://glotorrents.pw:6969/announce&tr=udp://tracker.opentrackr.org:1337/announce&tr=udp://torrent.gresille.org:80/announce&tr=udp://p4p.arenabg.com:1337&tr=udp://tracker.leechers-paradise.org:6969",
        hash, encoded_title
    )
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
            println!("✅ Database is up to date! No new movies to fetch.\n");
            return Ok(());
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
        let response = fetch_page(page)?;
        
        if let Some(movies) = response.data.movies {
            for api_movie in movies {
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
                        size_bytes: t.size_bytes,
                        magnet_url: magnet,
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

fn list_movies() -> Result<()> {
    let movies = load_existing_movies()?;
    
    if movies.is_empty() {
        println!("❌ No movies found. Run 'fetch' command first.");
        return Ok(());
    }
    
    println!("\n📚 Movie List ({} total)\n", movies.len());
    println!("{:<8} {:<50} {:<6} {:<15} {:<10}", "ID", "Title", "Year", "IMDb", "Torrents");
    println!("{}", "=".repeat(95));
    
    for movie in movies.iter().take(50) {
        let title = if movie.title.len() > 47 {
            format!("{}...", &movie.title[..47])
        } else {
            movie.title.clone()
        };
        
        println!(
            "{:<8} {:<50} {:<6} {:<15} {:<10}",
            movie.id,
            title,
            movie.year,
            movie.imdb_code,
            movie.torrents.len()
        );
    }
    
    if movies.len() > 50 {
        println!("\n... and {} more movies", movies.len() - 50);
    }
    
    Ok(())
}

fn show_stats() -> Result<()> {
    let movies = load_existing_movies()?;
    
    if movies.is_empty() {
        println!("❌ No movies found. Run 'fetch' command first.");
        return Ok(());
    }
    
    let total_torrents: usize = movies.iter().map(|m| m.torrents.len()).sum();
    let avg_torrents = total_torrents as f64 / movies.len() as f64;
    
    let mut years: Vec<u32> = movies.iter().map(|m| m.year).collect();
    years.sort();
    let oldest_year = years.first().unwrap_or(&0);
    let newest_year = years.last().unwrap_or(&0);
    
    println!("\n📊 Database Statistics\n");
    println!("Total Movies:        {}", movies.len());
    println!("Total Torrents:      {}", total_torrents);
    println!("Avg Torrents/Movie:  {:.2}", avg_torrents);
    println!("Year Range:          {} - {}", oldest_year, newest_year);
    println!("Latest Movie ID:     {}", movies.first().map(|m| m.id).unwrap_or(0));
    println!("Oldest Movie ID:     {}", movies.last().map(|m| m.id).unwrap_or(0));
    
    Ok(())
}

fn calculate_size() -> Result<()> {
    let movies = load_existing_movies()?;
    
    if movies.is_empty() {
        println!("❌ No movies found. Run 'fetch' command first.");
        return Ok(());
    }
    
    println!("\n💾 Calculating total size (using largest torrent per movie)...\n");
    
    let mut total_size: u64 = 0;
    let mut movies_with_torrents = 0;
    
    for movie in &movies {
        if let Some(max_torrent) = movie.torrents.iter().max_by_key(|t| t.size_bytes) {
            total_size += max_torrent.size_bytes;
            movies_with_torrents += 1;
        }
    }
    
    println!("📊 Size Statistics:\n");
    println!("Movies analyzed:     {}", movies.len());
    println!("Movies with torrents: {}", movies_with_torrents);
    println!("Total size:          {}", format_size(total_size));
    
    if movies_with_torrents > 0 {
        let avg_size = total_size / movies_with_torrents as u64;
        println!("Average per movie:   {}", format_size(avg_size));
    }
    
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Fetch => fetch_movies()?,
        Commands::List => list_movies()?,
        Commands::Stats => show_stats()?,
        Commands::Size => calculate_size()?,
    }
    
    Ok(())
}