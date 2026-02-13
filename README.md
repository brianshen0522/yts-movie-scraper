# YTS Movie Scraper

A Rust-based toolkit to scrape and manage YTS movie torrents with progress tracking and size calculations.

## Features

- 🎬 **Fetch Movies**: Download all YTS movies on first run, only new movies on subsequent runs
- 📋 **List Movies**: Display movies from your local database
- 🔢 **Count New Movies**: Check how many new movies are available without downloading
- 💾 **Calculate Size**: Get total combined size of all movies (using biggest torrent per movie)
- ⚡ **Progress Bar**: Visual progress tracking during downloads
- 📦 **Smart Storage**: Saves quality type (e.g., "1080p-web"), hash, magnet URL, and file size

## Installation

1. Clone or download this repository
```bash
git clone https://github.com/brianshen0522/yts-movie-scraper
```

2. Navigate to the project directory:
```bash
cd yts-movie-scraper
```

3. Build the project:
```bash
cargo build --release
```

## Usage

### 1. Fetch Movies (Default)

**First run** - Downloads all movies from YTS:
```bash
cargo run --release
# OR
cargo run --release fetch
```

**Subsequent runs** - Only downloads new movies:
```bash
cargo run --release
```

Output:
```
🎬 YTS Movie Scraper - Fetch Mode

📊 Fetching movie count...
Total movies in YTS: 73025

⠁ [00:00:15] [########>---------] 15234/73025 movies (00:02:45)
```

### 2. List Movies

Display movies from your database:
```bash
# Show first 10 movies (default)
cargo run --release list

# Show first 50 movies
cargo run --release list --limit 50
```

Output:
```
📊 Total movies in database: 73025

Showing first 10 movies:

1. [ID: 74251] Junun (2015)
   IMDb: tt4995590
   Torrents: 2
     - 720p-web (496.06 MB)
     - 1080p-web (920.62 MB)
```

### 3. Count New Movies

Check how many new movies are available without downloading:
```bash
cargo run --release count
```

Output:
```
🔍 Checking for new movies...

📁 Movies in local database: 73025
🆕 New movies available: 25
```

### 4. Calculate Total Size

Calculate combined size of all movies (biggest torrent per movie):
```bash
cargo run --release size
```

Output:
```
📊 Calculating total size (biggest torrent per movie)...

📦 Total movies: 73025
💾 Combined size: 124.5 TB
📊 Average size per movie: 1.74 GB
```

## Output Format

Movies are saved in `yts_movies.json`:
```json
[
  {
    "id": 74246,
    "title": "Love Me, Love Me",
    "year": 2026,
    "imdb_code": "tt36331860",
    "torrents": [
      {
        "quality": "720p-web",
        "hash": "C0EDF0F169275D7D889DEE3C073122B26FDFACA0",
        "magnet_url": "magnet:?xt=urn:btih:C0EDF0F169275D7D889DEE3C073122B26FDFACA0&dn=Love+Me,+Love+Me&tr=...",
        "size_bytes": 964815749
      },
      {
        "quality": "1080p-web",
        "hash": "A15EB9763B17540F9369E393E0074DB42B4A19D0",
        "magnet_url": "magnet:?xt=urn:btih:A15EB9763B17540F9369E393E0074DB42B4A19D0&dn=Love+Me,+Love+Me&tr=...",
        "size_bytes": 1975684956
      }
    ]
  }
]
```

## Command Reference

| Command | Description | Example |
|---------|-------------|---------|
| `fetch` | Download all/new movies | `cargo run --release fetch` |
| `list` | Show movies in database | `cargo run --release list --limit 20` |
| `count` | Count new available movies | `cargo run --release count` |
| `size` | Calculate total storage needed | `cargo run --release size` |

## Help

View all available commands:
```bash
cargo run --release -- --help
```

View help for a specific command:
```bash
cargo run --release list --help
```

## License

MIT