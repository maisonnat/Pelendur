pub mod auto_learn;
pub mod company;
pub mod company_research;
pub mod cv_parser;
pub mod embeddings;
pub mod graph;
pub mod graph_search;
pub mod migration;
pub mod personal;
pub mod post_interview_summary;
pub mod practice;
pub mod search;
pub mod skills;
pub mod star_matcher;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
