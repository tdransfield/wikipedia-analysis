
use std::io;
use std::fs::File;
use clap::{Arg, App, SubCommand};

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
            .subcommand(SubCommand::with_name("incoming-link-histogram")
                .about("List the number of articles with a given number of incoming links")
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
            .subcommand(SubCommand::with_name("print-steps")
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
            .subcommand(SubCommand::with_name("print-step-groups")
                .about("Print the articles grouped by depth away from the root article")
                .arg(Arg::with_name("depth")
                    .short("d")
                    .long("depth")
                    .takes_value(true)
                    .help("Maximum depth of article tree to evaluate")
                )
                .arg(Arg::with_name("roots")
                    .short("r")
                    .long("roots")
                    .takes_value(true)
                    .required(true)
                    .multiple(true)
                    .help("Root articles to evaluate step groups from (supports multiple)")
                )
            )
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("parse") {
        let (mut map, mut articles) = parse::parse_xml_dump(
            &matches
                .value_of("input")
                .expect("Input must be given")
                .to_string(),
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

        let mut output = match matches.value_of("output") {
            Some(filename) => Box::new(File::create(filename).unwrap()) as Box<dyn io::Write>,
            None => Box::new(io::stdout()) as Box<dyn io::Write>
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

        if let Some(matches) = matches.subcommand_matches("most-linked") {
            let count: usize = match matches.value_of("count").unwrap().parse().unwrap() {
                0 => analysis.articles.len(),
                x => x
            };

            let link_counts = analysis.get_most_incoming_links(count);
            writeln!(output, "incoming link count,number of articles with count").unwrap();
            for (index, (article_name, count)) in link_counts.iter().enumerate() {
                writeln!(output, "{}, \"{}\", {}", index, article_name, count).unwrap();
            }
        }

        else if let Some(_matches) = matches.subcommand_matches("incoming-link-histogram") {
            let link_counts = analysis.get_incoming_links_histogram();
            writeln!(output, "incoming link count,number of articles with count").unwrap();
            for (index, count) in link_counts.iter().enumerate() {
                writeln!(output, "{}, {}", index, count).unwrap();
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

        else if let Some(matches) = matches.subcommand_matches("print-steps") {

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
                Some(count) => writeln!(output, "Step count: {}", count.join(",")).unwrap(),
                None => writeln!(output, "No path from start to destination found").unwrap()
            };
        }

        else if let Some(matches) = matches.subcommand_matches("print-step-groups") {
            let depth = match matches.value_of("depth") {
                Some(match_value) => Some(match_value.parse().unwrap()),
                None => None
            };

            writeln!(
                output,
                "Article name\tincoming links (depth 0)\tincoming links (depth 1)\t...").unwrap();

            for root_article in matches.values_of("roots").unwrap() {
                let root_article_index = match analysis.article_map.get(root_article) {
                    Some(index) => index,
                    None => {
                        println!("Article with name '{}' not found", root_article);
                        return;
                    }
                };
                let step_groups = analysis.get_step_count_groups(
                    *root_article_index, depth
                );
                let steps_strs: Vec<String> = step_groups
                    .iter()
                    .map(|x| x.len().to_string())
                    .collect();
                writeln!(output, "{}\t{}", root_article, steps_strs.join("\t")).unwrap();
            }
        }
    }
    else {
        print!("{}", matches.usage());
    }
}