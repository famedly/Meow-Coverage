//! Helpers for operations on the GitHub API that are unsuported by [octocrab]

use hyper::{header::ACCEPT, http::HeaderValue, HeaderMap};
use octocrab::params::repos::Reference;
use serde::Deserialize;

/// Create a review comment on a PR
pub async fn create_review_comment(
	owner: &str,
	repo: &str,
	pull_id: u64,
	commit_id: &str,
	path: &str,
	first_line: u32,
	final_line: u32,
) -> Result<(), octocrab::Error> {
	let route = format!("/repos/{}/{}/pulls/{}/comments", owner, repo, pull_id);

	let body = match first_line == final_line {
		true => serde_json::json!({
			"body": "🐈‍⬛ Untested Line 🐈‍⬛",
			"commit_id": commit_id,
			"path": path,
			"start_side": "RIGHT",
			"line": final_line,
			"side": "RIGHT"
		}),
		false => serde_json::json!({
			"body": "🐈‍⬛ Untested Lines 🐈‍⬛",
			"commit_id": commit_id,
			"path": path,
			"start_line": first_line,
			"start_side": "RIGHT",
			"line": final_line,
			"side": "RIGHT"
		}),
	};

	let _: serde_json::Value = octocrab::instance().post(route, Some(&body)).await?;

	Ok(())
}

/// Wrapper to grab `sha` from response
#[derive(Debug, Deserialize)]
struct ShaWrapper {
	/// File blob SHA
	pub sha: String,
}

/// Create a review comment on a PR
pub async fn get_file_sha(
	owner: &str,
	repo: &str,
	reference: Reference,
	path: &str,
) -> Result<String, octocrab::Error> {
	let route =
		format!("/repos/{owner}/{repo}/contents/{path}", owner = owner, repo = repo, path = path,);

	let mut headers = HeaderMap::new();
	headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github.v3"));

	let value: ShaWrapper = octocrab::instance()
		.get_with_headers(route, Some(&[("ref", reference.ref_url())]), Some(headers))
		.await?;

	Ok(value.sha)
}
