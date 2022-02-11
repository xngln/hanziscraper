use anyhow::Result;
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::process;
use reqwest;
use scraper;

#[derive(Debug)]
struct HanziRow {
	hanzi: String,
	gs_num: u32,
	hsk_lvl: u32,
	freq: u32,	
}

struct Config {
	output_file_path: String,
	max_page: u32,
	starting_page: u32,
	base_url: String,
}

fn main() -> Result<(), reqwest::Error> {
    println!("Starting crawler");

	// config
	let args: Vec<String> = env::args().collect();
	let config = Config::new(&args).unwrap_or_else(|err| {
		println!("Problem parsing arguments: {}", err);
		process::exit(1);
	});

	println!("{}", config.output_file_path);
	scrape(config);

	Ok(())
}

fn scrape(config: Config) {
/*
	let file = OpenOptions::new().append(true)
								 .create(true)
								 .open(output_file_path);

	// loop through each 'page' of the table
	for page in starting_page..max_page {
		let url = base_url.to_owned() + &page.to_string();
		println!("{}", url);
		let res = reqwest::blocking::get(url)?.text()?;
		let html = scraper::Html::parse_document(&res);

		
	}
	*/

}

impl Config {
	fn new(args: &[String]) -> Result<Config, Box<dyn Error>> {
		// make sure enough arguments are provided
		if args.len() < 2 {
			return Err("not enough arguments".into());
		}

		let output_file_path: String= args[1].clone();
		// make sure the given path is ok
		let _ = OpenOptions::new().write(true).create(true).open(&output_file_path)?;

		let max_page: u32 = 3;
		let starting_page: u32 = 1;
		let base_url: String = String::from("http://hanzidb.org/character-list/by-frequency?page=");

		Ok(Config {
			output_file_path,
			max_page,
			starting_page,
			base_url,
		})
	}
}
