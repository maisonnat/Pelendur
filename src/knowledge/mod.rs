pub mod auto_learn;
pub mod cv_parser;
pub mod embeddings;
pub mod graph;
pub mod graph_search;
pub mod migration;
pub mod personal;
pub mod practice;
pub mod search;
pub mod skills;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
