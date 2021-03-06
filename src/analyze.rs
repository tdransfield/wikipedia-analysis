
use std::collections::HashMap;

use crate::parse::Article;
use std::convert::TryInto;
use std::mem;

/// Implements functions for analysing the parsed wikipedia data.
pub struct WikipediaAnalysis {
    /// A HashMap of article name -> article index
    pub article_map: HashMap<String, u32>,
    /// An adjacency list representation of the incoming links to each article.
    pub articles: Vec<Article>
}


impl WikipediaAnalysis {

    /// Initialises a vector with the given default up to the index.
    fn vec_initialise_up_to_index<T: Clone>(vec: &mut Vec<T>, index: usize, default: T) {
        while index >= vec.len() {
            vec.push(default.clone());
        }
    }

    /// Gets a sorted list of the pages with the most links.
    ///
    /// # Arguments
    /// * `count` - Number of items to return
    ///
    /// # Returns
    /// A sorted vector of tuples of (article index, link count).
    /// The vector is of length `count` unless `count` exceeds the number of articles.
    ///
    pub fn get_most_links(&self, count: u32) -> Vec<(u32, u32)> {
        let mut link_counts_map = Vec::new();
        for (_article_name, index) in self.article_map.iter() {
            link_counts_map.push(
                (*index, self.articles[*index as usize].links.len().try_into().unwrap()));
        }
        link_counts_map.sort_unstable_by(|a:&(u32, u32), b: &(u32, u32)| b.1.cmp(&a.1));
        return link_counts_map[0..count as usize].to_vec();
    }

    /// Gets a histogram of the number of links per page.
    ///
    /// # Returns
    /// Returns a vector indexed by the number of links.
    /// Values are the number of pages with that number of links.
    pub fn get_links_histogram(&self) -> Vec<u32> {
        let mut link_counts: Vec<u32> = Vec::new();
        for article in self.articles.iter() {
            WikipediaAnalysis::vec_initialise_up_to_index(
                &mut link_counts,
                article.links.len(),
                0
            );
            link_counts[article.links.len()] += 1;
        }
        return link_counts;
    }

    /// Gets the number of steps between two articles.
    /// Steps refers to points on the path of links between the articles.
    ///
    /// Note: Argument names are intended for incoming link representation. If you are using an
    /// input file that was generated as outgoing links then the arguments are reversed,
    ///
    /// # Arguments
    /// * `start_article` - The article to start stepping from
    /// * `destination_article` - The article to reach
    ///
    /// # Remarks
    /// This does not return the path itself, use `get_path_between_articles()` to get the path.
    ///
    /// # Returns
    /// The number of steps between the two articles.
    /// If no path is found None is returned.
    ///
    pub fn get_number_of_steps_between_articles(
        &self,
        start_article: u32,
        destination_article: u32) -> Option<u32> {

        // Perform a breadth-first-search for destination article from start article
        // BFS guarantees shortest path
        let mut depth = 1;
        let mut current_article_stack: Vec<u32> = Vec::new();
        let mut next_article_stack: Vec<u32> = Vec::new();
        let mut starting_links = self.articles[destination_article as usize].links.clone();
        current_article_stack.append(&mut starting_links);

        loop {
            for article_index in current_article_stack.drain(..) {
                if article_index == start_article {
                    return Some(depth);
                }
                next_article_stack.extend(self.articles[article_index as usize].links.iter());

            }
            current_article_stack.extend(next_article_stack.iter());
            if current_article_stack.len() == 0 {
                break;
            }
            depth += 1;
        }
        return None;
    }

    /// Gets the path between two articles.
    /// Steps refers to points on the path of links between the articles.
    ///
    /// Note: Argument names are intended for incoming link representation. If you are using an
    /// input file that was generated as outgoing links then the arguments are reversed,
    ///
    /// # Arguments
    /// * `start_article` - The article to start stepping from
    /// * `destination_article` - The article to reach
    ///
    /// # Remarks
    /// If you only need to establish if a path exists and/or how long it is, use
    /// `get_number_of_steps_between_articles()` instead as it is faster and lower in memory usage.
    ///
    /// # Returns
    /// A vec with the name of article steps between the two articles.
    /// If no path is found None is returned.
    ///
    pub fn get_path_between_articles(
        &self,
        start_article: u32,
        destination_article: u32) -> Option<Vec<u32>> {

        // Perform a breadth-first-search for destination article from start article
        // BFS guarantees shortest path
        let mut current_article_stack: Vec<Vec<u32>> = Vec::new();
        let mut next_article_stack: Vec<Vec<u32>> = Vec::new();

        // See `get_step_count_groups()` for how this works
        const BITS_PER_BYTE: usize = 8;
        const BITMASK: usize = BITS_PER_BYTE * mem::size_of::<usize>() - 1;
        const LOG2_BITS_PER_USIZE: usize = BITMASK.count_ones() as usize;
        let mut visited: Vec<usize> = vec![0; (self.articles.len() >> LOG2_BITS_PER_USIZE as usize) + 1];

        for article in self.articles[destination_article as usize].links.iter() {
            current_article_stack.push(vec!(*article));
            visited[*article as usize >> LOG2_BITS_PER_USIZE] |= 1 << (*article as usize & BITMASK);
        }

        loop {
            for article_path in current_article_stack.drain(..) {
                let current_article = article_path[article_path.len() - 1];

                for next_article in self.articles[current_article as usize].links.iter() {

                    // First time this is seen is guaranteed to be
                    // the shortest (or equal-shortest) route
                    if *next_article == start_article {
                        let mut path = Vec::with_capacity(article_path.len() + 1);

                        // Generate output vec in correct order
                        path.push(start_article);
                        for index in article_path.iter().rev() {
                            path.push(*index);
                        }
                        path.push(destination_article);
                        return Some(path);
                    }

                    if (visited[*next_article as usize >> LOG2_BITS_PER_USIZE] & 1 << (*next_article as usize & BITMASK)) == 0 {
                        let mut next_path = article_path.clone();
                        next_path.push(*next_article);
                        next_article_stack.push(next_path);
                        visited[*next_article as usize >> LOG2_BITS_PER_USIZE] |= 1 << (*next_article as usize & BITMASK);
                    }
                }
            }
            current_article_stack.extend(next_article_stack.drain(..));
            if current_article_stack.len() == 0 {
                break;
            }
        }
        return None;
    }

    /// Gets a list of articles at each step from the starting article.
    ///
    /// Steps count groups refers to the articles of step n from the starting article.
    /// Eg. for a link graph like: a <= b <= c with start article a, a would be in step group 0,
    /// b would be in step group 1 and c would be in step group 2.
    ///
    /// # Arguments
    /// * `root_article` - The root of the incoming link tree.
    /// * `max_depth` - The maximum depth of the incoming link tree to evaluate.
    ///
    /// # Remarks
    /// If you only need to establish if a path exists and/or how long it is, use
    /// `get_number_of_steps_between_articles()` instead as it is faster and lower in memory usage.
    ///
    /// # Returns
    /// A vec indexed by the step count group, containing items which are
    /// a vec of the indices of incoming links th that group.
    ///
    /// Eg in the example:
    /// ```
    ///     a
    ///    /  \
    ///   b    c
    ///         \
    ///          d
    /// ```
    /// Where the indices of a, b, c, d are 0, 1, 2, 3
    /// the result would be [[0], [1, 2], [3]].
    ///
    pub fn get_step_count_groups(
        &self,
        root_article: u32,
        max_depth: Option<u32>) -> Vec<Vec<u32>> {

        let root_article = root_article as usize;

        let mut depth = match max_depth {
            Some(depth) => depth,
            None => self.articles.len().try_into().unwrap()
        };
        let mut groups: Vec<Vec<u32>> = Vec::new();
        groups.push( self.articles[root_article].links.clone());

        // Array to check if a node has been visited
        // As this is a bitfield we are going to pack the bits as tightly as possible
        // and use usize as the native size for accessing it (assumed fastest size for basic integer
        // operations and memory alignment on the architecture)
        // This helps to reduce cache pressure as this is a fairly hot area of memory
        const BITS_PER_BYTE: usize = 8;
        const BITMASK: usize = BITS_PER_BYTE * mem::size_of::<usize>() - 1;
        const LOG2_BITS_PER_USIZE: usize = BITMASK.count_ones() as usize;

        let mut visited: Vec<usize> = vec![0; (self.articles.len() >> LOG2_BITS_PER_USIZE as usize) + 1];

        // Initialise visited elements
        // Visited is set before expanding a node to avoid having
        // multiple of the same nodes in the to visit group.
        visited[root_article >> LOG2_BITS_PER_USIZE] |= 1 << (root_article & BITMASK);
        for next_article in self.articles[root_article].links.iter() {
            visited[*next_article as usize >> LOG2_BITS_PER_USIZE] |= 1 << (*next_article as usize & BITMASK);
        }

        while depth > 1 {
            let current_article_stack = &groups[groups.len() - 1];
            let mut next_article_stack: Vec<u32> = Vec::new();
            for current_article in current_article_stack.iter() {
                for next_article in self.articles[*current_article as usize].links.iter() {
                    // Check if article has been visited
                    if (visited[*next_article as usize >> LOG2_BITS_PER_USIZE] & 1 << (*next_article as usize & BITMASK)) == 0 {
                        next_article_stack.push(*next_article);
                        // Mark article visited
                        visited[*next_article as usize >> LOG2_BITS_PER_USIZE] |= 1 << (*next_article as usize & BITMASK);
                    }
                }
            }

            // No articles left to add
            if next_article_stack.len() == 0 {
                break;
            }
            groups.push(next_article_stack);
            depth -= 1;
        }
        return groups;
    }
}
