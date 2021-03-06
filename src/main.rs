
use std::io;
use std::fs::File;
use clap::{Arg, App, SubCommand};
use rand::{Rng, thread_rng};
use std::collections::HashMap;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::io::{BufReader, BufRead};
use num_cpus;
use std::cmp;
use std::convert::TryInto;

pub mod parse;
pub mod analyze;

/// Entry point for CLI parser
fn main() {
    let matches = App::new("Wikipedia link graph analysis tool")
        .version("0.1.0")
        .author("Tom Dransfield")
        .about("Parses and analyses wikipedia XML dumps.")
        .subcommand(SubCommand::with_name("parse")
            .about("Parse XML dump into an intermediate format")
            .arg(Arg::with_name("input")
                .short("i")
                .long("input")
                .required(true)
                .takes_value(true)
                .index(1)
                .help("Wikipedia XML dump file to parse")
            )
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .default_value("_processed_wikipedia_dump.tsv")
                .help("Output intermediate file")
            )
            .arg(Arg::with_name("ignore")
                .short("n")
                .long("ignore-dir")
                .takes_value(true)
                .help("Path to a directory containing textfiles that \
                    are a list of article names to ignore")
            )
            .arg(Arg::with_name("reverse")
                .short("r")
                .long("reverse")
                .takes_value(false)
                .help("Reverse the intermediate file format to be a list of outgoing links \
                          instead of a list of incoming links")
            )
        )
        .subcommand(SubCommand::with_name("analyze")
            .about("Analyse using an intermediate file")
            .arg(Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .default_value("_processed_wikipedia_dump.tsv")
                .help("Input intermediate file to use for analysis")
            )
            .arg(Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Output results file (defaults to STDOUT)")
            )
            .subcommand(SubCommand::with_name("most-linked")
                .about("List the files most commonly linked to")
                .arg(Arg::with_name("count")
                    .short("c")
                    .long("count")
                    .takes_value(true)
                    .default_value("0")
                    .help("Number of items to list")
                )
            )
            .subcommand(SubCommand::with_name("link-histogram")
                .about("List the number of articles with a given number of links")
            )
            .subcommand(SubCommand::with_name("links")
                .about("Print the names of articles which link to the start article")
                .arg(Arg::with_name("start")
                    .short("s")
                    .long("start")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Name of article to start from")
                )
            )
            .subcommand(SubCommand::with_name("count-steps")
                .about("Count the number of steps between two articles, from start to destination")
                .arg(Arg::with_name("start")
                    .short("s")
                    .long("start")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Name of article to start from")
                )
                .arg(Arg::with_name("destination")
                    .short("d")
                    .long("destination")
                    .takes_value(true)
                    .required(true)
                    .index(2)
                    .help("Name of article to find step count to")
                )
            )
            .subcommand(SubCommand::with_name("steps")
                .about("Print the articles between two articles, from start to destination")
                .arg(Arg::with_name("start")
                    .short("s")
                    .long("start")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Name of article to start from")
                )
                .arg(Arg::with_name("destination")
                    .short("d")
                    .long("destination")
                    .takes_value(true)
                    .required(true)
                    .index(2)
                    .help("Name of article to find step count to")
                )
            )
            .subcommand(SubCommand::with_name("step-groups")
                .about("Print the articles grouped by depth away from the root article")
                .arg(Arg::with_name("depth")
                    .short("d")
                    .long("depth")
                    .takes_value(true)
                    .help("Maximum depth of article tree to evaluate")
                )
                .arg(Arg::with_name("roots")
                    .long("roots")
                    .takes_value(true)
                    .required(false)
                    .multiple(true)
                    .conflicts_with_all(&["roots-file", "use-most-linked", "use-random"])
                    .help("Root articles to evaluate step groups from (supports multiple).")
                )
                .arg(Arg::with_name("roots-file")
                    .long("roots-file")
                    .takes_value(true)
                    .required(false)
                    .conflicts_with_all(&["roots", "use-most-linked", "use-random"])
                    .help("Use a file with a list of roots to evaluate (separated by newline).")
                )
                .arg(Arg::with_name("use-most-linked")
                    .long("use-most-linked")
                    .takes_value(true)
                    .required(false)
                    .help("Use the top n most linked articles as the roots. \
                              Set to zero to use all articles")
                )
                .arg(Arg::with_name("use-random")
                    .long("use-random")
                    .takes_value(true)
                    .required(false)
                    .help("Use n randomly selected articles")
                )
                .arg(Arg::with_name("num-threads")
                    .short("j")
                    .long("num-threads")
                    .takes_value(true)
                    .required(false)
                    .help("Number of worker threads to use for parallel processing. Defaults to \
                          the number of physical CPU cores -1 (or 1 for single core systems).")
                )
            )
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("parse") {

        let to_ignore = match matches.value_of("ignore") {
            Some(ignore_path) => Some(parse::parse_ignore_directory(&ignore_path.to_string())),
            None => None
        };

        let mode = match matches.is_present("reverse") {
            true => parse::ParserMode::OutgoingLinks,
            false => parse::ParserMode::IncomingLinks
        };

        let (mut map, mut articles) = parse::parse_xml_dump(
            &matches
                .value_of("input")
                .expect("Input must be given")
                .to_string(),
            to_ignore,
            mode
        );

        parse::write_to_tsv(
            &matches
            .value_of("output")
            .expect("Output must be given")
            .to_string(),
            &mut map,
            &mut articles
        );
    }

    else if let Some(matches) = matches.subcommand_matches("analyze") {

        let mut output: Box<dyn io::Write + Send> = match matches.value_of("output") {
            Some(filename) => Box::new(File::create(filename).unwrap()),
            None => Box::new(io::stdout())
        };

        let (lookup_table, adjacency_list) = parse::load_from_tsv(
            &matches
                .value_of("input")
                .expect("Input must be given")
                .to_string(),
        );

        let analysis = analyze::WikipediaAnalysis {
            article_map: lookup_table,
            articles: adjacency_list
        };

        let index_map = generate_index_lookup_table(&analysis.article_map);

        if let Some(matches) = matches.subcommand_matches("most-linked") {
            let count: u32 = match matches.value_of("count").unwrap().parse().unwrap() {
                0 => analysis.articles.len().try_into().unwrap(),
                x => x
            };

            let link_counts = analysis.get_most_links(count);
            writeln!(output, "position\tarticle name\tcount").unwrap();
            for (index, (article_index, count)) in link_counts.iter().enumerate() {
                let article_name = index_map[*article_index as usize];
                writeln!(output, "{}\t{}\t{}", index, article_name, count).unwrap();
            }
        }

        else if let Some(_matches) = matches.subcommand_matches("link-histogram") {
            let link_counts = analysis.get_links_histogram();
            writeln!(output, "link count\tnumber of articles with count").unwrap();
            for (index, count) in link_counts.iter().enumerate() {
                writeln!(output, "{}\t{}", index, count).unwrap();
            }
        }

        else if let Some(matches) = matches.subcommand_matches("links") {
            let start_article = matches.value_of("start").unwrap();
            let start_article_index = match analysis.article_map.get(start_article) {
                Some(index) => index,
                None => {
                    println!("Article with name '{}' not found", start_article);
                    return;
                }
            };
            let articles = &analysis.articles[*start_article_index as usize].links;
            for article_index in articles.iter() {
                writeln!(output, "{}", index_map[*article_index as usize]).unwrap();
            }
        }

        else if let Some(matches) = matches.subcommand_matches("count-steps") {

            let start_article = matches.value_of("start").unwrap();
            let destination_article = matches.value_of("destination").unwrap();
            let start_article_index = match analysis.article_map.get(start_article) {
                Some(index) => index,
                None => {
                    println!("Article with name '{}' not found", start_article);
                    return;
                }
            };
            let destination_article_index = match analysis.article_map.get(destination_article) {
                Some(index) => index,
                None => {
                    println!("Article with name '{}' not found", destination_article);
                    return;
                }
            };

            let path = analysis.get_number_of_steps_between_articles(
                *start_article_index, *destination_article_index
            );
            match path {
                Some(count) => writeln!(output, "Path: {}", count).unwrap(),
                None => println!("No path from start to destination found")
            };
        }

        else if let Some(matches) = matches.subcommand_matches("steps") {

            let start_article = matches.value_of("start").unwrap();
            let destination_article = matches.value_of("destination").unwrap();
            let start_article_index = match analysis.article_map.get(start_article) {
                Some(index) => index,
                None => {
                    println!("Article with name '{}' not found", start_article);
                    return;
                }
            };
            let destination_article_index = match analysis.article_map.get(destination_article) {
                Some(index) => index,
                None => {
                    println!("Article with name '{}' not found", destination_article);
                    return;
                }
            };

            let step_count = analysis.get_path_between_articles(
                *start_article_index, *destination_article_index
            );
            match step_count {
                Some(count) => {
                    let article_names: Vec<String> = count
                        .iter()
                        .map(|x| index_map[*x as usize].clone())
                        .collect();
                    writeln!(output, "Step count: {}", article_names.join(",")).unwrap();
                },
                None => {
                    writeln!(output, "No path from start to destination found").unwrap();
                }
            };
        }

        else if let Some(matches) = matches.subcommand_matches("step-groups") {
            let depth = match matches.value_of("depth") {
                Some(match_value) => Some(match_value.parse().unwrap()),
                None => None
            };

            writeln!(
                output,
                "Article name\tlinks (depth 0)\tlinks (depth 1)\t...").unwrap();

            let mut roots: Vec<u32> = Vec::new();
            if matches.is_present("use-most-linked") {
                let count: u32 = matches.value_of("use-most-linked").unwrap().parse().unwrap();
                roots = analysis.get_most_links(count)
                    .iter()
                    .map(|x| x.0)
                    .collect();
            }
            else if matches.is_present("use-random") {
                let count: u32 = matches.value_of("use-random").unwrap().parse().unwrap();
                let mut rng = thread_rng();
                for _ in 0..count {
                    let article_index = rng.gen_range(0, analysis.articles.len());
                    roots.push(article_index.try_into().unwrap());
                }
            }
            else if matches.is_present("roots") {
                for article in matches.values_of("roots").unwrap() {
                    match analysis.article_map.get(article) {
                        Some(article_index) => {
                            roots.push(*article_index);
                        },
                        None => {
                            println!("Article with name '{}' not found", article);
                        }
                    };
                };
            }
            else if matches.is_present("roots-file") {
                let filename = matches.value_of("roots-file").unwrap();
                let file = File::open(filename).unwrap();
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let article = line.unwrap();
                    match analysis.article_map.get(&article) {
                        Some(article_index) => {
                            roots.push(*article_index);
                        },
                        None => {
                            println!("Article with name '{}' not found", article);
                        }
                    };
                }
            }
            else {
                println!("Must use one of: [use-most-linked, random, roots]");
                return;
            }

            let write_mutex = Arc::new(Mutex::new(output));

            let steps_function = |root_article_index| {
                let step_groups = analysis.get_step_count_groups(
                    root_article_index, depth
                );
                let steps_strs: Vec<String> = step_groups
                    .iter()
                    .map(|x| x.len().to_string())
                    .collect();
                let root_article_name = index_map[root_article_index as usize];

                let mut mutex = write_mutex.lock().unwrap();
                writeln!(mutex, "{}\t{}", root_article_name, steps_strs.join("\t")).unwrap();
            };

            // Set number of worker threads
            let num_threads = match matches.value_of("num-threads") {
                Some(thread_count) => thread_count.parse::<usize>().unwrap(),
                None => cmp::max(1, num_cpus::get_physical() - 1)
            };
            rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().unwrap();

            roots.into_par_iter().for_each(steps_function);
        }
    }
    else {
        print!("{}", matches.usage());
    }
}

/// Generates a hashmap from article index -> article name
fn generate_index_lookup_table(article_map: &HashMap<String, u32>) -> Vec<&String> {
    unsafe {
        let mut index_lookup_table = Vec::with_capacity(article_map.len());
        index_lookup_table.set_len(article_map.len());
        for (article_name, index) in article_map.iter() {
            index_lookup_table[*index as usize] = article_name;
        }
        return index_lookup_table;
    }

}
