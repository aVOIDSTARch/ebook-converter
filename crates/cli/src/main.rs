use clap::{Parser, Subcommand};
use std::path::Path;

use ebook_converter_core::config::{config_path, load_config, AppConfig};
use ebook_converter_core::convert::{convert_path, parse_format, read_document, write_document};
use ebook_converter_core::cover::extract_cover;
use ebook_converter_core::dedup::{find_duplicates, DuplicateStrategy};
use ebook_converter_core::detect::detect_file;
use ebook_converter_core::merge;
use ebook_converter_core::meta;
use ebook_converter_core::optimize;
use ebook_converter_core::readers::ReadOptions;
use ebook_converter_core::repair;
use ebook_converter_core::rename;
use ebook_converter_core::split::{split, SplitStrategy};
use ebook_converter_core::validate::{validate, ValidateOptions, WcagLevel};
use ebook_converter_core::writers::WriteOptions;
use ebook_converter_core::lookup::openlibrary::OpenLibraryProvider;
use ebook_converter_core::lookup::{MetadataProvider, MetadataQuery};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

#[derive(Parser)]
#[command(name = "ebook-converter")]
#[command(about = "Ebook format conversion, validation, and repair")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert between ebook formats
    Convert {
        /// Input file(s)
        #[arg(required = true)]
        input: Vec<String>,

        /// Output file or directory
        #[arg(short, long)]
        output: Option<String>,

        /// Output format (epub, txt, html, md, pdf, ssml)
        #[arg(short, long)]
        format: Option<String>,

        /// Rename output using format string
        #[arg(long)]
        rename: Option<String>,
    },

    /// Validate ebook structure
    Validate {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,

        /// Run accessibility checks
        #[arg(long)]
        accessibility: bool,

        /// WCAG level (A, AA, AAA)
        #[arg(long, default_value = "AA")]
        wcag_level: String,
    },

    /// Show ebook info and metadata
    Info {
        /// Input file
        #[arg(required = true)]
        input: String,
    },

    /// Repair ebook issues
    Repair {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Optimize ebook size
    Optimize {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,

        /// Image quality (1-100)
        #[arg(long, default_value = "80")]
        quality: u8,
    },

    /// Rename ebook files using template
    Rename {
        /// Input file(s)
        #[arg(required = true)]
        input: Vec<String>,

        /// Format template string
        #[arg(long, required = true)]
        template: String,

        /// Preview changes without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Output directory
        #[arg(long)]
        outdir: Option<String>,
    },

    /// Edit ebook metadata
    Meta {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Get a metadata field
        #[arg(long)]
        get: Option<String>,

        /// Set a metadata field (field=value)
        #[arg(long)]
        set: Option<Vec<String>>,

        /// Strip metadata (optionally specify fields)
        #[arg(long)]
        strip: bool,
    },

    /// Extract cover image
    Cover {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Output image path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Look up metadata from online providers
    Lookup {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Provider name
        #[arg(long, default_value = "openlibrary")]
        provider: String,

        /// Apply found metadata to the file
        #[arg(long)]
        apply: bool,
    },

    /// Merge multiple ebooks into one
    Merge {
        /// Input files
        #[arg(required = true, num_args = 2..)]
        inputs: Vec<String>,

        /// Output file
        #[arg(short, long, required = true)]
        output: String,
    },

    /// Split an ebook into parts
    Split {
        /// Input file
        #[arg(required = true)]
        input: String,

        /// Split strategy (chapter, heading, pages)
        #[arg(long, default_value = "chapter")]
        by: String,

        /// Output directory
        #[arg(long)]
        outdir: Option<String>,
    },

    /// Find duplicate ebooks
    Dedup {
        /// Input files or directories
        #[arg(required = true)]
        inputs: Vec<String>,

        /// Strategy (hash, isbn, fuzzy, content)
        #[arg(long, default_value = "fuzzy")]
        strategy: String,

        /// Similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.85")]
        threshold: f64,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Initialize default config file
    Init,
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Key (dot-separated path)
        key: String,
        /// Value
        value: String,
    },
}

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    let result = match &cli.command {
        Commands::Convert { input, output, format, rename: _ } => run_convert(input, output.as_deref(), format.as_deref(), cli.json),
        Commands::Validate { input, strict, accessibility, wcag_level } => run_validate(input, *strict, *accessibility, wcag_level, cli.json),
        Commands::Info { input } => run_info(input, cli.json),
        Commands::Repair { input, output } => run_repair(input, output.as_deref(), cli.json),
        Commands::Optimize { input, output, quality } => run_optimize(input, output.as_deref(), *quality, cli.json),
        Commands::Rename { input, template, dry_run, outdir } => run_rename(input, template, *dry_run, outdir.as_deref(), cli.json),
        Commands::Meta { input, get, set, strip } => run_meta(input, get.as_deref(), set.as_deref(), *strip, cli.json),
        Commands::Cover { input, output } => run_cover(input, output.as_deref(), cli.json),
        Commands::Lookup { input, provider, apply } => run_lookup(input, provider, *apply, cli.json),
        Commands::Merge { inputs, output } => run_merge(inputs, output, cli.json),
        Commands::Split { input, by, outdir } => run_split(input, by, outdir.as_deref(), cli.json),
        Commands::Dedup { inputs, strategy, threshold } => run_dedup(inputs, strategy, *threshold, cli.json),
        Commands::Config { action } => run_config(action, cli.json),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_convert(
    inputs: &[String],
    output: Option<&str>,
    format_str: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let read_opts = ReadOptions::default();
    let write_opts = WriteOptions::default();

    let output_format = format_str
        .and_then(parse_format)
        .unwrap_or(ebook_converter_core::detect::Format::Epub);

    for input_path in inputs {
        let input_path = Path::new(input_path);
        if !input_path.exists() {
            eprintln!("Input file not found: {}", input_path.display());
            continue;
        }

        let _detected = detect_file(input_path)?;
        let out_path = if let Some(o) = output {
            Path::new(o).to_path_buf()
        } else {
            let stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            let ext = output_format.extension();
            input_path.parent().unwrap_or(Path::new(".")).join(format!("{}.{}", stem, ext))
        };

        if out_path.is_dir() {
            let stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            let out_path = out_path.join(format!("{}.{}", stem, output_format.extension()));
            convert_path(input_path, &out_path, output_format, &read_opts, &write_opts)?;
        } else {
            convert_path(input_path, &out_path, output_format, &read_opts, &write_opts)?;
        }

        if !json {
            println!("Converted: {} -> {}", input_path.display(), out_path.display());
        }
    }

    Ok(())
}

fn run_validate(
    input: &str,
    strict: bool,
    accessibility: bool,
    wcag_level: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut header = vec![0u8; 4096];
    let n = reader.read(&mut header)?;
    header.truncate(n);
    reader.seek(SeekFrom::Start(0))?;
    let filename = path.file_name().and_then(|p| p.to_str());
    let detected = ebook_converter_core::detect::detect(&header, filename)?;
    let doc = read_document(detected.format, reader, &ReadOptions::default(), None)?;

    let opts = ValidateOptions {
        strict,
        accessibility,
        wcag_level: WcagLevel::from_str(wcag_level),
    };
    let issues = validate(&doc, &opts);

    if json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
    } else {
        for issue in &issues {
            let severity = format!("{:?}", issue.severity);
            println!("[{}] {}: {}", severity, issue.code, issue.message);
        }
        if issues.is_empty() {
            println!("Validation passed.");
        } else {
            let errors = issues.iter().filter(|i| matches!(i.severity, ebook_converter_core::validate::Severity::Error)).count();
            if strict && errors > 0 {
                return Err(format!("Validation failed with {} error(s)", errors).into());
            }
        }
    }
    Ok(())
}

fn run_repair(
    input: &str,
    output: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let doc = read_doc_from_path(path)?;
    let mut doc = doc;
    let report = repair::repair(&mut doc, &repair::RepairOptions::default());
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for a in &report.fixes_applied {
            println!("Fixed: {} - {}", a.code, a.description);
        }
        for (a, msg) in &report.fixes_failed {
            eprintln!("Failed: {} - {}", a.code, msg);
        }
    }
    if let Some(out) = output {
        let out_path = Path::new(out);
        let format = ebook_converter_core::detect::Format::Epub;
        let file = File::create(out_path)?;
        write_document(format, &doc, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
        if !json {
            println!("Wrote: {}", out_path.display());
        }
    }
    Ok(())
}

fn run_optimize(
    input: &str,
    output: Option<&str>,
    quality: u8,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let mut doc = read_doc_from_path(path)?;
    let opts = optimize::OptimizeOptions { image_quality: quality, ..optimize::OptimizeOptions::default() };
    let report = optimize::optimize(&mut doc, &opts);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Original: {} bytes, Optimized: {} bytes", report.original_size_bytes, report.optimized_size_bytes);
        for a in &report.actions {
            println!("  {}", a);
        }
    }
    if let Some(out) = output {
        let out_path = Path::new(out);
        let format = ebook_converter_core::detect::Format::Epub;
        let file = File::create(out_path)?;
        write_document(format, &doc, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
        if !json {
            println!("Wrote: {}", out_path.display());
        }
    }
    Ok(())
}

fn run_meta(
    input: &str,
    get: Option<&str>,
    set: Option<&[String]>,
    strip: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let mut doc = read_doc_from_path(path)?;

    if let Some(field) = get {
        if let Some(v) = meta::meta_get(&doc, field) {
            println!("{}", v);
        }
        return Ok(());
    }

    if strip {
        meta::meta_strip(&mut doc, None);
        if !json {
            println!("Stripped optional metadata");
        }
    }

    if let Some(pairs) = set {
        for pair in pairs {
            if let Some((k, v)) = pair.split_once('=') {
                meta::meta_set(&mut doc, k.trim(), v.trim())?;
            }
        }
    }

    if strip || set.is_some() {
        let format = ebook_converter_core::detect::Format::Epub;
        let file = File::create(path)?;
        write_document(format, &doc, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
        if !json {
            println!("Updated: {}", path.display());
        }
    }

    Ok(())
}

fn run_cover(
    input: &str,
    output: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let doc = read_doc_from_path(path)?;
    let (data, media_type) = extract_cover(&doc).ok_or("No cover image found")?;
    let ext = if media_type.contains("png") { "png" } else { "jpg" };
    let out_path = output.map(|s| Path::new(s).to_path_buf()).unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")).join(format!("cover.{}", ext)));
    std::fs::write(&out_path, &data)?;
    if !json {
        println!("Extracted cover to {}", out_path.display());
    }
    Ok(())
}

fn run_lookup(
    input: &str,
    _provider: &str,
    apply: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let mut doc = read_doc_from_path(path)?;
    let query = MetadataQuery {
        title: doc.metadata.title.clone(),
        author: doc.metadata.authors.first().cloned(),
        isbn: doc.metadata.isbn_13.clone().or(doc.metadata.isbn_10.clone()),
    };
    let provider = OpenLibraryProvider::new();
    let results = provider.search(&query)?;
    if results.is_empty() {
        if !json {
            println!("No results found");
        }
        return Ok(());
    }
    if apply {
        let r = &results[0];
        if let Some(t) = &r.title {
            doc.metadata.title = Some(t.clone());
        }
        if !r.authors.is_empty() {
            doc.metadata.authors = r.authors.clone();
        }
        if r.isbn_13.is_some() {
            doc.metadata.isbn_13 = r.isbn_13.clone();
        }
        if r.description.is_some() {
            doc.metadata.description = r.description.clone();
        }
        let file = File::create(path)?;
        write_document(ebook_converter_core::detect::Format::Epub, &doc, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
        if !json {
            println!("Applied metadata to {}", path.display());
        }
    } else {
        if json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            for (i, r) in results.iter().take(5).enumerate() {
                println!("Result {}: {} by {:?}", i + 1, r.title.as_deref().unwrap_or("?"), r.authors);
            }
        }
    }
    Ok(())
}

fn run_rename(
    inputs: &[String],
    template: &str,
    dry_run: bool,
    outdir: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for input in inputs {
        let path = Path::new(input);
        let doc = read_doc_from_path(path).ok();
        let new_name = rename::format_title(input, template, doc.as_ref().map(|d| &d.metadata))?;
        if json {
            println!("{}", new_name);
        } else if dry_run {
            println!("{} -> {}", input, new_name);
        } else {
            let out_path = outdir.map(|d| Path::new(d).join(&new_name)).unwrap_or_else(|| Path::new(&new_name).to_path_buf());
            std::fs::copy(path, &out_path)?;
            println!("Renamed to {}", out_path.display());
        }
    }
    Ok(())
}

fn run_merge(
    inputs: &[String],
    output: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut docs = Vec::new();
    for input in inputs {
        docs.push(read_doc_from_path(Path::new(input))?);
    }
    let merged = merge::merge(&docs, &merge::MergeOptions::default())?;
    let out_path = Path::new(output);
    let format = ebook_converter_core::detect::Format::Epub;
    let file = File::create(out_path)?;
    write_document(format, &merged, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
    if !json {
        println!("Merged {} files -> {}", docs.len(), out_path.display());
    }
    Ok(())
}

fn run_split(
    input: &str,
    by: &str,
    outdir: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let doc = read_doc_from_path(path)?;
    let strategy = match by.to_lowercase().as_str() {
        "chapter" => SplitStrategy::Chapter,
        "heading" => SplitStrategy::Heading(1),
        "pages" => SplitStrategy::PageCount(5000),
        _ => SplitStrategy::Chapter,
    };
    let docs = split(&doc, strategy)?;
    let base = outdir.unwrap_or(".").trim_end_matches('/');
    for (i, d) in docs.iter().enumerate() {
        let out_path = Path::new(base).join(format!("part_{}.epub", i + 1));
        let file = File::create(&out_path)?;
        write_document(ebook_converter_core::detect::Format::Epub, d, std::io::BufWriter::new(file), &WriteOptions::default(), None)?;
        if !json {
            println!("Wrote {}", out_path.display());
        }
    }
    Ok(())
}

fn run_dedup(
    inputs: &[String],
    strategy: &str,
    threshold: f64,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let paths: Vec<&Path> = inputs.iter().map(|s| Path::new(s.as_str())).collect();
    let strat = match strategy.to_lowercase().as_str() {
        "hash" => DuplicateStrategy::Hash,
        "fuzzy" => DuplicateStrategy::Fuzzy,
        _ => DuplicateStrategy::Fuzzy,
    };
    let groups = find_duplicates(&paths, strat, threshold)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&groups)?);
    } else {
        for g in &groups {
            println!("Duplicate group ({}):", g.paths.len());
            for p in &g.paths {
                println!("  {}", p.display());
            }
        }
    }
    Ok(())
}

fn read_doc_from_path(path: &Path) -> Result<ebook_converter_core::document::Document, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut header = vec![0u8; 4096];
    let n = reader.read(&mut header)?;
    header.truncate(n);
    reader.seek(SeekFrom::Start(0))?;
    let filename = path.file_name().and_then(|p| p.to_str());
    let detected = ebook_converter_core::detect::detect(&header, filename)?;
    read_document(detected.format, reader, &ReadOptions::default(), None).map_err(|e| e.into())
}

fn run_info(input: &str, json: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(input);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut header = vec![0u8; 4096];
    let n = reader.read(&mut header)?;
    header.truncate(n);
    reader.seek(SeekFrom::Start(0))?;

    let filename = path.file_name().and_then(|p| p.to_str());
    let detected = ebook_converter_core::detect::detect(&header, filename)?;

    let doc = read_document(
        detected.format,
        reader,
        &ReadOptions::default(),
        None,
    )?;

    if json {
        let info = serde_json::json!({
            "metadata": doc.metadata,
            "stats": doc.stats(),
            "format": format!("{:?}", detected.format),
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Format: {:?}", detected.format);
        if let Some(t) = &doc.metadata.title {
            println!("Title: {}", t);
        }
        if !doc.metadata.authors.is_empty() {
            println!("Authors: {}", doc.metadata.authors.join(", "));
        }
        let s = doc.stats();
        println!("Words: {}", s.word_count);
        println!("Chapters: {}", s.chapter_count);
        println!("Reading time: {:.1} min", s.estimated_reading_time_minutes);
    }

    Ok(())
}

fn run_config(
    action: &ConfigAction,
    json: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match action {
        ConfigAction::Init => {
            let path = config_path().ok_or("Could not determine config directory")?;
            std::fs::create_dir_all(path.parent().unwrap())?;
            let default_cfg = AppConfig::default();
            let toml = toml::to_string_pretty(&default_cfg)?;
            std::fs::write(&path, toml)?;
            println!("Wrote default config to {}", path.display());
        }
        ConfigAction::Show => {
            let cfg = load_config();
            if json {
                println!("{}", serde_json::to_string_pretty(&cfg)?);
            } else {
                println!("{}", toml::to_string_pretty(&cfg)?);
            }
        }
        ConfigAction::Set { key, value } => {
            let path = config_path().ok_or("Could not determine config directory")?;
            let mut cfg: AppConfig = if path.exists() {
                let s = std::fs::read_to_string(&path)?;
                toml::from_str(&s).unwrap_or_else(|_| AppConfig::default())
            } else {
                AppConfig::default()
            };

            set_config_key(&mut cfg, key, value)?;

            std::fs::create_dir_all(path.parent().unwrap())?;
            let toml = toml::to_string_pretty(&cfg)?;
            std::fs::write(&path, toml)?;
            if !json {
                println!("Updated {}", key);
            }
        }
    }
    Ok(())
}

fn set_config_key(cfg: &mut AppConfig, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    match parts.as_slice() {
        ["library", sub] => match *sub {
            "format" => cfg.library.format = value.to_string(),
            "template" => cfg.library.template = value.to_string(),
            "output_dir" => cfg.library.output_dir = Some(value.to_string()),
            _ => return Err(format!("Unknown key: {}", key).into()),
        },
        ["lookup", sub] => match *sub {
            "default_provider" => cfg.lookup.default_provider = Some(value.to_string()),
            "cache_dir" => cfg.lookup.cache_dir = Some(value.to_string()),
            "cache_ttl_hours" => cfg.lookup.cache_ttl_hours = value.parse().ok(),
            _ => return Err(format!("Unknown key: {}", key).into()),
        },
        ["security", sub] => match *sub {
            "max_file_size_mb" => cfg.security.max_file_size_mb = value.parse().ok(),
            "max_compression_ratio" => cfg.security.max_compression_ratio = value.parse().ok(),
            _ => return Err(format!("Unknown key: {}", key).into()),
        },
        _ => return Err(format!("Unknown key: {}", key).into()),
    }
    Ok(())
}
