use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs::File;
use std::io::*;
use std::collections::HashMap;
use regex::Regex;

// XML parsing state
enum ParserState {
    Idle,
    ReadingTitle,
    ReadingBody
}

pub struct Article {
    /// This is part of an adjacency list representation of the incoming link graph
    /// Incoming links are identified by their index in this vector
    /// Article names are not preserved.
    pub incoming_links: Vec<usize>
}

/// Approximate number of articles in the 2017_11_03 wikipedia XML dump
const NUM_ARTICLES: usize = 6_000_000;

/// Checks if a given title is 'valid' for my definition of valid in relation to this project.
///
/// Returns `true` if the title is valid, `false` otherwise.
///
/// # Arguments
/// * `title` - The page title with first character capitalized
///
/// # Remarks
/// In general a 'valid' page is an encyclopedia article, I try to avoid any meta pages relating
/// to wikipedia itself. I also try to avoid picture links and disambiguation pages.
///
/// Note that wikipedia links are case sensitive except for the first letter. It is preferred
/// that articles have the first letter capitalized to match the wikipedia style guide.
///
fn is_valid_title(title: &str) -> bool {
    if title.len() == 0 {
        return false;
    }
    if let Some(_) = title.find(":") {
        // Not the most efficient but doesn't take unreasonably
        // long for the moment as the parsing XML step should only be run once
        if title.starts_with("File") ||
            title.starts_with("Discussion") ||
            title.starts_with("Image") ||
            title.starts_with("Category") ||
            title.starts_with("Wikipedia") ||
            title.starts_with("Portal") ||
            title.starts_with("Template") ||
            title.starts_with("Draft") ||
            title.starts_with("User") ||
            title.starts_with("Commons") ||
            title.starts_with("Wikt") ||
            title.starts_with("Book") ||
            title.starts_with("Mediawiki") ||
            title.starts_with("User talk"){
            return false;
        }
    }
    if let Some(_) = title.find("\n") {
        return false
    }
    if let Some(_) = title.find("\t") {
        return false
    }
    if title.contains("(disambiguation)") {
        return false;
    }
    return true;
}


pub trait StringExt {
    /// Capitalize first letter to match wikipedia style
    fn capitalize_first_letter(&self) -> String;
}

impl StringExt for String {
    fn capitalize_first_letter(&self) -> String {
        let mut char_iter = self.chars();
        match char_iter.next() {
            None => String::new(),
            Some(chr) => chr.to_uppercase().collect::<String>() + char_iter.as_str()
        }
    }
}

/// Scans through pages in a given wikipedia XML dump and calls
/// the given callback for each valid page. A valid page is one
/// that passes the `is_valid_title()` check.
///
/// # Arguments
/// * `xml_path` - Path to the unprocessed XML database dump
/// * `valid_page_callback` - A callback that is executed for every valid page
///
fn scan_pages<F>(xml_path: &String, mut valid_page_callback: F) -> ()
    where F: FnMut(String, String){
    let file = File::open(xml_path).unwrap();
    let buf_reader = BufReader::new(file);
    let mut reader =  Reader::from_reader(buf_reader);

    let mut source_article_name: Option<String> = None;
    let mut parser_state = ParserState::Idle;

    loop {
        let mut buf = Vec::new();
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"title" => parser_state = ParserState::ReadingTitle,
                    b"text" => {
                        match source_article_name {
                            Some(_) => parser_state = ParserState::ReadingBody,
                            None => ()
                        }
                    },
                    _ => (),
                }
            }

            Ok(Event::Text(e)) => {
                match parser_state {
                    ParserState::ReadingTitle => {
                        // Wikipedia does not care about the case of
                        // the first letter in the title. Generally
                        // sentence case is preferred for article titles,
                        // so capitalize first letter if it is not already.
                        // We must do this because page links can appear
                        // as upper case or lower case.
                        let article_name = e.unescape_and_decode(&reader)
                            .unwrap()
                            .trim()
                            .to_string()
                            .capitalize_first_letter();

                        if is_valid_title(&article_name) {
                            source_article_name = Some(article_name);
                        }
                        else {
                            source_article_name = None;
                        }
                    }

                    ParserState::ReadingBody => {
                        let source_article_name = source_article_name
                            .take()
                            .expect("Article must be defined");

                        let body = e.unescape_and_decode(&reader).unwrap();

                        valid_page_callback(source_article_name, body);
                    },
                    _ => ()
                }
                parser_state = ParserState::Idle;
            },
            Ok(Event::End(_)) => {
                parser_state = ParserState::Idle
            },
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }
        buf.clear();
    }
}

/// Parses a wikipedia XML database dump into an adjacency list of incoming links.
///
/// # Arguments
/// * `xml_path` - Path to the unprocessed XML database dump
///
/// # Returns
///  * A HashMap of article name -> article index
///  * An adjacency list representation of the incoming links to each article.
///
/// # Panics
/// There are several potential panics from regexes relating to the format of text within the XML document.
/// This function has only been tested with the 2017-11-03 pages-articles-multistream XML dump and
/// does not panic with this dataset, but may with others if they do not follow the same format.
///
/// # Remarks
///
/// Info on the wikipedia XML format can be found [here](https://en.wikipedia.org/wiki/Wikipedia:Database_download).
/// The output of this function is only article names and a graph of links.
/// All other information is stripped (eg text).
/// For the database dump used for testing (_2017-11-03 pages-articles-multistream_)
/// a ~60GB file is reduced to ~1.2GB when serialized to TSV (using `write_to_tsv()`).
///
/// Some links are not added:
/// * Links inside infoboxes (the box on the right of a page, usually with information about places of interest)
/// * Links from disambiguation pages
///
/// This function performs two passes over the database dump. The first pass finds all valid pages
/// (including redirects). Before the second pass the redirects are 'forwarded' through the graph
/// until they point to a real page. For all links, if no real page is found to match then the link
/// is not added. In practise there are many more empty links than real page links.
/// 
/// The incoming link adjacency list representation was chosen as it makes later analysis much
/// easier to process (for my intended use cases). Parsing is harder as state must be maintained
/// during the parsing process, however this only needs to be done once then the result is saved
/// so this was a good compromise for my use case.
///
pub fn parse_xml_dump(xml_path: &String) -> (HashMap<String, usize>, Vec<Article>) {

    // Compile regexes once for efficiency
    let link_regex = Regex::new(r"\[\[([^\[\]]+)\]\]").unwrap();
    let infobox_regex = Regex::new(r"(?ms)\{\{Infobox.*?^\}\}").unwrap();
    let main_article_regex = Regex::new(r"\{\{main article\|([^{}\|]+?)\}\}").unwrap();
    let see_also_regex = Regex::new(r"\{\{see also\|([^\{\}]+?)\}\}").unwrap();

    // Maps name of article => index of Article struct in articles
    let mut article_map: HashMap<String, usize> = HashMap::with_capacity(NUM_ARTICLES);
    // Maps name of article to name of article to redirect to
    let mut redirect_to: HashMap<String, String> = HashMap::with_capacity(NUM_ARTICLES);
    let mut articles: Vec<Article> = Vec::with_capacity(NUM_ARTICLES);

    let get_valid_pages = | article_name: String, body: String | -> () {
        // Redirect pages must start with #redirect followed by
        // the page they are redirecting to. No other text is allowed.
        // Case of redirect doesnt matter but im assuming no one will
        // do anything silly like rEdIrEcT
        let is_redirect =
            body.starts_with("#redirect") ||
            body.starts_with("#REDIRECT");

        // https://simple.wikipedia.org/wiki/MediaWiki:Disambiguationspage
        // This should cover most uses
        let is_disambiguation =
            body.contains("{{disamb") ||
            body.contains("{{Disamb") ||
            body.contains("{{dab}}");

        if is_redirect && link_regex.is_match(&body){
            // If the page is a redirect then there is one outgoing link
            // to the page any incoming links should be redirected to
            let redirected_to_article_name: String = link_regex
                .captures(&body)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .split("|").next().unwrap()  // Select article name
                .split("#").next().unwrap()       // Strip in page anchor
                .trim()
                .to_string()
                .capitalize_first_letter();

            if is_valid_title(&redirected_to_article_name) {
                let insert_result = redirect_to.insert(
                    article_name.clone(),
                    redirected_to_article_name.clone()
                );
                match insert_result {
                    Some(old) => {
                        println!(
                            "Multiple page redirects, should not happen: {}: {}, {}",
                            article_name,
                            old,
                            redirected_to_article_name
                        );
                    },
                    None => ()
                }
            }
        }

        // Normal article page
        else if !is_disambiguation {
            let article_name = article_name;
            match article_map.get(&article_name) {
                Some(_) => println!("Multiple page insertions, should not happen: {}", article_name),
                None => {
                    article_map.insert(
                        article_name,
                        article_map.len()
                    );
                    articles.push(Article {
                        incoming_links: Vec::new()
                    });
                }
            }
        }
    };

    scan_pages(xml_path, get_valid_pages);

    // Finally parse articles again for their outgoing links
    // Place each outgoing link as an incoming link in the graph with
    // the source being the current article and the destination being the
    // article link found in the current article's body
    // Any links to redirects are redirected towards the real article after
    // following the redirects
    let redirects_map = resolve_redirects(&article_map, &mut redirect_to);

    let add_links = | article_name: String, body: String | -> () {

        let source_article_index = match article_map.get(&article_name) {
            Some(source_article_index) => source_article_index,
            None => return
        };

        // Skip infobox if present
        let infobox = infobox_regex.shortest_match(&body);
        let body = match infobox {
            Some(end_position) => &body[end_position..],
            None => &body
        };

        // Article links are of the form:
        // [[article name#optional_anchor|display name]]
        let mut outgoing_links: Vec<String> = link_regex
            .captures_iter(body)
            .map(|x| x
                .get(1)
                .unwrap()
                .as_str()
                .split("|").next().unwrap()  // Select article name
                .split("#").next().unwrap()       // Strip in page anchor
                .trim()
                .to_string()
                .capitalize_first_letter())
            .collect();

        for capture in main_article_regex.captures_iter(&body) {
            let link = capture
                .get(1)
                .unwrap()
                .as_str()
                .split("#").next().unwrap()       // Strip in page anchor
                .trim()
                .to_string();
            outgoing_links.push(link.to_string());
        }

        for capture in see_also_regex.captures_iter(&body) {
            for link in capture.get(1).unwrap().as_str().split("|") {
                outgoing_links.push(link.split("#").next().unwrap().trim().to_string());
            }
        }

        outgoing_links.dedup();

        // Add the incoming links to any destination pages
        for link_title in outgoing_links {

            let dest_article_index = article_map
                .get(&link_title)
                .or(redirects_map
                    .get(&link_title));

            match dest_article_index {
                Some(dest_article_index) => {
                    articles[*dest_article_index].incoming_links.push(*source_article_index);
                },
                None => ()
            }
        }
    };

    scan_pages(xml_path, add_links);

    return (article_map, articles)
}

/// Takes in the values returned by `parse_xml_to_tsv()` and writes them to a TSV file.
///
/// The TSV format produced consists of only a unique sequential integer index
/// for each article, the article name and then a list of article indices with a link to this article.
///
/// # Arguments
/// * `output_path` - File path to write the TSV output to
/// * `article_map` - Hashmap of article name -> article index
/// * `articles` - Adjacency list representation of incoming links graph
///
pub fn write_to_tsv(
    output_path: &String,
    article_map: &mut HashMap<String, usize>,
    articles: &mut Vec<Article>) -> () {

    println!("{}, {}",article_map.len(), articles.len());
    assert_eq!(article_map.len(), articles.len());

    // Convert hashmap to vec in correct order based on index
    let mut article_titles: Vec<Option<String>> = Vec::with_capacity(NUM_ARTICLES);
    for _ in 0..article_map.len() {
        article_titles.push(None);
    }

    for (title, index) in article_map.iter() {
        article_titles[*index] = Some(title.clone());
    }

    let mut fout_links_graph = File::create(output_path).unwrap();

    for article_index in 0..articles.len() {
        let article_name = article_titles[article_index]
            .as_ref()
            .expect("Title index defined");

        articles[article_index].incoming_links.dedup();
        let links_string: String = articles[article_index].incoming_links
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("\t");

        fout_links_graph
            .write(format!("{}\t{}\t{}\n",
                           article_index,
                           article_name,
                           links_string).as_bytes())
            .unwrap();
    }
}

/// Loads a TSV (produced by `write_to_tsv()`) back into hashmap and adjacency list representation.
///
/// # Arguments
/// * `tsv_path` - Path to the TSV file to load
///
/// # Returns
///  * A HashMap of article name -> article index
///  * An adjacency list representation of the incoming links to each article.
///
/// # Panics
/// May panic if the TSV file becomes corrupted
///
pub fn load_from_tsv(tsv_path: &String) -> (HashMap<String, usize>, Vec<Article>) {
    let file = File::open(tsv_path).unwrap();
    let reader = BufReader::new(file);

    let mut lookup_table: HashMap<String, usize> = HashMap::with_capacity(NUM_ARTICLES);
    let mut adjacency_list: Vec<Article> = Vec::with_capacity(NUM_ARTICLES);

    for line in reader.lines() {
        let line = line.unwrap();
        let fields: Vec<&str> = line.split("\t").collect();

        // TSV has at least 2 fields:
        // Index \t Article name \t Redirect field \t article exists \t Incoming link indicies
        if fields.len() >= 2 {
            let article_index = fields[0].parse::<usize>().unwrap();
            let article_title = fields[1].to_string();

            // There should not be duplicate articles in the TSV
            assert_eq!(lookup_table.insert(article_title, article_index), None);

            // Index should match line number (0-indexed)
            // If they do not match then we have skipped data
            // and the adjacency list indexes will be wrong
            assert_eq!(adjacency_list.len(), article_index);

            let incoming_links = match fields[2].len() > 0 {
                true => fields[2..]
                        .iter()
                        .map(|x| x.parse::<usize>().unwrap())
                        .collect(),
                false => Vec::new()
            };

            adjacency_list.push(Article {
                incoming_links
            });
        }
    }
    return (lookup_table, adjacency_list);
}

/// Recursively resolves redirected article links to find the actual article they link to.
///
/// Most redirects are only a single step, however there is a small number that
/// take multiple steps. Some redirect links may not resolve to an actual article and are discarded.
///
/// # Arguments
/// * `article_map` - Hashmap of article name -> article index
/// * `redirects` - Hashmap of article name -> article name (to be redirected to)
///
/// # Returns
/// * A HashMap of article name -> article index, mapping redirected articles to indices
///
fn resolve_redirects(
    article_map: &HashMap<String, usize>,
    redirects: &mut HashMap<String, String>) -> HashMap<String, usize> {

    let mut redirects_map: HashMap<String, usize> = HashMap::with_capacity(NUM_ARTICLES);

    for (curr_article_name, redirected_to_article_name) in redirects.iter() {
        let mut current_redirect_article_name = redirected_to_article_name;
        while article_map.get(current_redirect_article_name) == None {
            if let Some(next_redirect) = redirects.get(current_redirect_article_name) {
                current_redirect_article_name = next_redirect;
            }
            else {
                // Found a dead link
                // No matching redirect and no matching article
                break;
            }
        }

        if let Some(redirect_to_index) = article_map.get(redirected_to_article_name) {
            redirects_map.insert(curr_article_name.clone(), *redirect_to_index);
        }
    }
    return redirects_map;
}
