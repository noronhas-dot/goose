use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
    collect_baseline_metrics, metrics_hashmap_to_vec, write_response_to_file, EvalMetricValue,
    Evaluation, ExtensionRequirements,
};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct AppleContainerSearch {}

impl AppleContainerSearch {
    pub fn new() -> Self {
        AppleContainerSearch {}
    }

    fn check_repository_mentions(&self, text: &str) -> bool {
        // Check if the response mentions Apple and container-related repositories
        let text_lower = text.to_lowercase();
        let has_apple = text_lower.contains("apple");
        let has_container = text_lower.contains("container") || text_lower.contains("docker");

        has_apple && has_container
    }

    fn count_repositories(&self, text: &str) -> i64 {
        // Count repository mentions with various patterns
        let mut count = 0;
        count += text.matches("github.com/apple/").count();
        count += text.matches("https://github.com/apple/").count();
        count += text.matches("http://github.com/apple/").count();
        // Also match apple/repo-name pattern (but be careful not to double-count)
        let simple_pattern = text.matches("apple/").count();
        // Subtract the ones we already counted to avoid duplicates
        let duplicates = text.matches("github.com/apple/").count()
            + text.matches("https://github.com/apple/").count()
            + text.matches("http://github.com/apple/").count();
        count += simple_pattern.saturating_sub(duplicates);
        count as i64
    }
}

#[async_trait]
impl Evaluation for AppleContainerSearch {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("AppleContainerSearch - run");

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (response, perf_metrics) = collect_baseline_metrics(
            agent,
            "Search for Apple repositories related to containers on GitHub. Find repositories from the Apple organization that deal with container technology, Docker, or containerization. Provide a list with repository names and descriptions. Use your available tools to search for these repositories.".to_string()
        ).await;

        // Write response to file and get the text content
        let response_text = match write_response_to_file(
            response.messages(),
            run_loc,
            "apple_container_search_output.txt",
        ) {
            Ok(text) => text,
            Err(e) => {
                println!(
                    "Warning: Failed to write apple container search output: {}",
                    e
                );
                // If file write fails, still continue with the evaluation
                response
                    .last()
                    .map_or_else(String::new, |msg| msg.as_concat_text())
            }
        };

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if the response mentions relevant repositories
        let has_relevant_mentions = self.check_repository_mentions(&response_text);
        let repository_count = self.count_repositories(&response_text);

        metrics.push((
            "has_relevant_mentions".to_string(),
            EvalMetricValue::Boolean(has_relevant_mentions),
        ));
        metrics.push((
            "repository_count".to_string(),
            EvalMetricValue::Integer(repository_count),
        ));

        // Check if GitHub search tools were used - look for specific tool patterns
        let used_github_search =
            crate::eval_suites::used_tool(response.messages(), "search_repositories")
                || crate::eval_suites::used_tool(response.messages(), "search_code")
                || crate::eval_suites::used_tool(response.messages(), "list_");
        metrics.push((
            "used_github_search".to_string(),
            EvalMetricValue::Boolean(used_github_search),
        ));

        // Calculate a simple success score
        let score = if has_relevant_mentions && repository_count > 0 {
            1.0
        } else if has_relevant_mentions || repository_count > 0 {
            0.5
        } else {
            0.0
        };

        metrics.push(("score".to_string(), EvalMetricValue::Float(score)));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "apple_container_search"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec![],
            external: vec![],
            remote: vec!["github".to_string()],
        }
    }
}

register_evaluation!(AppleContainerSearch);
