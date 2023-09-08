//! Helpers for building comments in HTML
use std::borrow::Cow;

use itertools::Itertools;

use super::{PullFileCoverageWrapper, PushFileCoverageWrapper};

/// Makes a file, and optionally, line specific link to a diff in a PR
pub fn make_pull_link(
	owner: &str,
	repo: &str,
	pull_id: u64,
	sha: &str,
	line: Option<(u32, Option<u32>)>,
) -> String {
	format!(
		"https://github.com/{}/{}/pull/{}/files#diff-{}{}",
		owner,
		repo,
		pull_id,
		sha,
		match line {
			Some((start_line, Some(end_line))) =>
				Cow::Owned(format!("R{}-R{}", start_line, end_line)),
			Some((line, None)) => Cow::Owned(format!("R{}", line)),
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
	line: Option<(u32, Option<u32>)>,
) -> String {
	format!(
		"https://github.com/{}/{}/commit/{}#diff-{}{}",
		owner,
		repo,
		commit_sha,
		file_name_sha,
		match line {
			Some((start_line, Some(end_line))) =>
				Cow::Owned(format!("R{}-R{}", start_line, end_line)),
			Some((line, None)) => Cow::Owned(format!("R{}", line)),
			None => Cow::Borrowed(""),
		}
	)
}

/// Gather lines that are next to each other
fn gather_lines(lines: &[u32]) -> Vec<(u32, u32)> {
	lines.iter().sorted().copied().fold(Vec::<(u32, u32)>::new(), |mut acc, val| {
		if acc.iter().any(|&(low, high)| val >= low && val <= high) {
			return acc;
		}

		match acc.last_mut() {
			Some(last) => {
				if val != 0 && last.1 == val - 1 {
					last.1 = val;
				} else {
					acc.push((val, val));
				}
			}
			None => acc.push((val, val)),
		}

		acc
	})
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_gather_lines() {
		use super::gather_lines;

		assert_eq!(gather_lines(&[0, 1, 2, 3, 5, 6, 8, 10]), &[(0, 3), (5, 6), (8, 8), (10, 10)]);
		assert_eq!(gather_lines(&[3, 1, 4, 5, 6, 8, 10]), &[(1, 1), (3, 6), (8, 8), (10, 10)]);
		assert_eq!(gather_lines(&[]), &[]);
		assert_eq!(gather_lines(&[0, 0, 1, 0]), &[(0, 1)]);
	}
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
                        itertools::intersperse(gather_lines(&file_cov.raw_lines).into_iter().map(|(start_line, end_line)| {
							Cow::Owned(match start_line == end_line {
								true => {
									html_to_string_macro::html! {
										<a href={make_commit_link(owner, repo, commit_sha, file_cov.sha.as_str(), Some((start_line, None)))}>{start_line}</a>
									}
								},
								false => {
									html_to_string_macro::html! {
										<a href={make_commit_link(owner, repo, commit_sha, file_cov.sha.as_str(), Some((start_line, Some(end_line))))}>{start_line}"-"{end_line}</a>
									}
								},
							})
						}), Cow::Borrowed(", "))
						.collect::<String>()
                    }
                </td>
            </tr>
        }
    }).collect())
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
                        itertools::intersperse(gather_lines(&file_cov.raw_lines).into_iter().map(|(start_line, end_line)| {
							Cow::Owned(match start_line == end_line {
								true => {
									html_to_string_macro::html! {
										<a href={make_pull_link(owner, repo, pull_id, file_cov.sha.as_str(), Some((start_line, None)))}>{start_line}</a>
									}
								},
								false => {
									html_to_string_macro::html! {
										<a href={make_pull_link(owner, repo, pull_id, file_cov.sha.as_str(), Some((start_line, Some(end_line))))}>{start_line}"-"{end_line}</a>
									}
								},
							})
						}), Cow::Borrowed(", "))
						.collect::<String>()
                    }
                </td>
            </tr>
        }
    }).collect())
}
