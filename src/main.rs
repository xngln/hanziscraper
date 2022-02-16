use anyhow::{Result, anyhow};
use opencc::OpenCC;
use pinyin::ToPinyinMulti;
use reqwest;
use scraper;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::process;

#[derive(Debug)]
struct HanziRow {
    hanzi: String,
    trad: String,
    kanji: String,
    pinyin: String,
    hsk_lvl: u32,
    gs_num: u32,
    freq: u32,	
}

impl fmt::Display for HanziRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}	{}	{}	{}	{}	{}	{}\n", self.hanzi, self.trad, self.kanji, self.pinyin, self.hsk_lvl, self.gs_num, self.freq)
    }
}

struct Config {
    output_file_path: String,
    max_page: u32,
    starting_page: u32,
    base_urls: Vec<String>,
}

impl Config {
    fn new(args: &[String]) -> Result<Config> {
        // make sure enough arguments are provided
        if args.len() != 2 {
            return Err(anyhow!("Incorrect number of arguments provided."));
        }

        let output_file_path: String= args[1].clone();
        // make sure the given path is ok
        let _ = OpenOptions::new().write(true).create(true).open(&output_file_path)?;

        // hardcode these for now
        let max_page: u32 = 101;
        let starting_page: u32 = 1;
        let base_url_freq: String = String::from("http://hanzidb.org/character-list/by-frequency?page=");
        let base_url_gs: String = String::from("http://hanzidb.org/character-list/general-standard?page=");
        let base_urls: Vec<String> = vec![base_url_freq, base_url_gs];

        Ok(Config {
            output_file_path,
            max_page,
            starting_page,
            base_urls,
        })
    }
}

fn main() -> Result<(), reqwest::Error> {
    println!("Starting crawler");

    // config
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        print_usage();
        process::exit(1);
    });

    println!("Writing data to {}.\n", config.output_file_path);
    match scrape(config) {
        Ok(num) => println!("Successfully finished crawling HanziDB. Found {} hanzi.", num),
        Err(e) => println!("Something went wrong while scraping: {}", e),
    }
    Ok(())
}

struct Selectors {
    tr: scraper::Selector,
    td: scraper::Selector,	
    a: scraper::Selector,
}

impl Selectors {
    fn new() -> Result<Selectors> {
        let tr = scraper::Selector::parse("tr");
        let tr = match tr {
            Ok(selector) => selector,
            Err(_) => return Err(anyhow!("Selector parsing failed."))
        };
        let td = scraper::Selector::parse("td");
        let td = match td {
            Ok(selector) => selector,
            Err(_) => return Err(anyhow!("Selector parsing failed."))
        };
        let a = scraper::Selector::parse("a");
        let a = match a {
            Ok(selector) => selector,
            Err(_) => return Err(anyhow!("Selector parsing failed."))
        };

        Ok(Selectors {
            tr,
            td,
            a,
        })
    }
}

fn scrape(config: Config) -> Result<u32> {
    let mut num_lines_written: u32 = 0;
    let mut file = OpenOptions::new().append(true)
                                 .create(true)
                                 .open(config.output_file_path)?;

    let selectors = Selectors::new()?;
    let cc_s2t= OpenCC::new("s2t.json"); // simplified to traditional
    let cc_t2jp = OpenCC::new("t2jp.json"); // traditional to japanese (shinjitai)
    let mut hanzi_map: HashMap<String, bool> = HashMap::new();

    // loop through each 'page' of the table
    for base_url in config.base_urls {
        for page in config.starting_page..config.max_page {
            let url = base_url.to_owned() + &page.to_string();
            println!("{}", url);
            let res = reqwest::blocking::get(url)?;
            if !res.status().is_success() {
                if page > 82 {
                    return Ok(num_lines_written);
                }	
                return Err(anyhow!("Failed to get page."));
            }
            let html = scraper::Html::parse_document(&res.text()?);

            // loop through the rows in each table page. each row corresponds to a character.
            for row in html.select(&selectors.tr) {
                let mut data = row.select(&selectors.td);
                let hanzi: String;
                let trad: String;
                let kanji: String;
                let pinyin: String;
                let hsk_lvl: u32;
                let gs_num: u32;
                let freq: u32;

                // get the character by getting
                // html structure looks like: <td><a href="some_link">字</a></td>
                match data.nth(0) {
                    Some(hz_container) => {
                        match hz_container.select(&selectors.a).next() {
                            Some(hz_element) => {
                                match hz_element.text().next() {
                                    Some(hz) => { 
                                        hanzi = hz.to_string(); 
                                        if hanzi_map.contains_key(&hanzi) { continue; }
                                    },
                                    None => continue,
                                };
                            },
                            None => continue,
                        };
                    },
                    None => continue,
                };

                // get the hsk level
                match data.nth(4) {
                    Some(hsk_container) => {
                        match hsk_container.text().next() {
                            Some(hsk) => { 
                                hsk_lvl = match hsk.parse::<u32>() {
                                    Ok(uint) => uint,
                                    Err(_) => return Err(anyhow!("Failed string to u32 conversion.")),
                                }; 
                            },
                            None => hsk_lvl = 0, // we want to keep the characters even if they aren't in the hsk. 
                        }
                    },
                    None => continue,
                };

                // get the general standard #
                // if the character is not in the gs, I don't want to keep it (for now). 
                // most likely it is a rarely used variant or traditional character.
                match data.next() {
                    Some(gs_container) => {
                        match gs_container.text().next() {
                            Some(gs) => {
                                gs_num = match gs.parse::<u32>() {
                                    Ok(uint) => uint,
                                    Err(_) => return Err(anyhow!("Failed string to u32 conversion.")),
                                };
                            },
                            None => continue,
                        }
                    },
                    None => continue,
                };

                // get the frequency rank
                match data.next() {
                    Some(freq_container) => {
                        match freq_container.text().next() {
                            Some(f) => {
                                freq = match f.parse::<u32>() {
                                    Ok(uint) => uint,
                                    Err(_) => return Err(anyhow!("Failed string to u32 conversion.")),
                                };
                            },
                            None => freq = 0, // if no frequency, still keep the character
                        }
                    },
                    None => continue,
                }

                // get pinyin readings
                if let Some(x) = hanzi.as_str().to_pinyin_multi().next() {
                    if let Some(p) = x {
                        let readings: Vec<&str> = p.into_iter().map(|x| x.with_tone()).collect();
                        pinyin = readings.join(", ");
                    } else { return Err(anyhow!("Failed to get pinyin")); }
                } else { return Err(anyhow!("Failed to get pinyin")); }

                // get traditional character
                trad = cc_s2t.convert(&hanzi);

                // get kanji (shinjitai)
                kanji = cc_t2jp.convert(&trad);

                // write data to file
                let data = HanziRow {
                    hanzi: hanzi.clone(),
                    trad,
                    kanji,
                    pinyin,
                    hsk_lvl,
                    gs_num,
                    freq,
                };
                match file.write(format!("{}", data).as_bytes()) {
                    Ok(_) => {
                        num_lines_written = num_lines_written + 1;
                        hanzi_map.insert(hanzi, true);
                    },
                    Err(e) => return Err(anyhow!("Got error while writing to file: {}", e)),
                }
            }

        }
    }

    Ok(num_lines_written)
}

fn print_usage() {
    println!("Usage: ./hanziscraper <output_file_path>");
}
