//! Helpers for building comments in HTML
use std::borrow::Cow;

use crate::{PullFileCoverageWrapper, PushFileCoverageWrapper};

/// Makes a file, and optionally, line specific link to a diff in a PR
pub fn make_pull_link(
	owner: &str,
	repo: &str,
	pull_id: u64,
	sha: &str,
	line: Option<u32>,
) -> String {
	format!(
		"https://github.com/{}/{}/pull/{}/files#diff-{}{}",
		owner,
		repo,
		pull_id,
		sha,
		match line {
			Some(line) => Cow::Owned(format!("R{}", line)),
			None => Cow::Borrowed(""),
		}
	)
}

/// Makes a file, and optionally line, specific link for a commit diff
pub fn make_commit_link(
	owner: &str,
	repo: &str,
	commit_sha: &str,
	file_name_sha: &str,
	line: Option<u32>,
) -> String {
	format!(
		"https://github.com/{}/{}/commit/{}#diff-{}{}",
		owner,
		repo,
		commit_sha,
		file_name_sha,
		match line {
			Some(line) => Cow::Owned(format!("R{}", line)),
			None => Cow::Borrowed(""),
		}
	)
}

/// Internal summary builder
fn build_summary(summary: &str, table_rows: String) -> String {
	html_to_string_macro::html! {
		<details>
			<summary>{ summary }</summary>
			<table>
				<tbody>
					<tr>
						<th>"File Path"</th>
						<th>"Lines"</th>
					</tr>
					{ table_rows }
				</tbody>
			</table>
		</details>
	}
}

/// Build comment summary for a commit in HTML
pub fn build_push_summary(
	owner: &str,
	repo: &str,
	commit_sha: &str,
	report: &[PushFileCoverageWrapper],
) -> String {
	build_summary("🐈‍⬛ Untested Lines 🐈‍⬛", report.iter().map(|file_cov|  {
        html_to_string_macro::html! {
            <tr>
                <td>
                <a href={make_commit_link(owner, repo, commit_sha, file_cov.sha.as_str(), None)}>{file_cov.realpath.as_str()}</a>
                </td>
                <td>
                    {
                        itertools::intersperse(file_cov.raw_lines.iter().map(|num| Cow::Owned(html_to_string_macro::html! {
                            <a href={make_commit_link(owner, repo, commit_sha, file_cov.sha.as_str(), Some(*num))}>{num}</a>
                        })), Cow::Borrowed(", ")).fold(String::new(), |l, r| l + r.as_ref())
                    }
                </td>
            </tr>
        }
    }).fold(String::new(), |l, r| l + r.as_ref()))
}

/// Build comment summary for a PR in HTML
pub fn build_pull_summary(
	owner: &str,
	repo: &str,
	pull_id: u64,
	report: &[PullFileCoverageWrapper],
) -> String {
	build_summary("🐈‍⬛ Untested Changes 🐈‍⬛", report.iter().map(|file_cov|  {
        html_to_string_macro::html! {
            <tr>
                <td>
                <a href={make_pull_link(owner, repo, pull_id, file_cov.sha.as_str(), None)}>{file_cov.realpath.as_str()}</a>
                </td>
                <td>
                    {
                        itertools::intersperse(file_cov.raw_lines.iter().map(|num| Cow::Owned(html_to_string_macro::html! {
                            <a href={make_pull_link(owner, repo, pull_id, file_cov.sha.as_str(), Some(*num))}>{num}</a>
                        })), Cow::Borrowed(", ")).fold(String::new(), |l, r| l + r.as_ref())
                    }
                </td>
            </tr>
        }
    }).fold(String::new(), |l, r| l + r.as_ref()))
}
