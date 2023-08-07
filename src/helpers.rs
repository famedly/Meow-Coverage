//! General helper utils

/// Split a path by `source_prefix`, retaining the splitter in the right-paw
/// side
pub fn path_split(path: &str, source_prefix: &str) -> String {
	path.split_once(source_prefix)
		.map_or_else(|| String::from(path), |(_, val)| format!("{}{}", source_prefix, val))
}

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

/// Check if a line was changed in a [patch::Hunk]
pub fn line_changed_in_hunk(hunk: &patch::Hunk, target_line: u64) -> bool {
	let mut current_line = hunk.new_range.start;

	if target_line < current_line || target_line >= current_line + hunk.new_range.count {
		return false;
	}

	hunk.lines.iter().any(|line| match line {
		patch::Line::Add(_) => match current_line == target_line {
			true => true,
			false => {
				current_line += 1;
				false
			}
		},
		patch::Line::Context(_) => {
			current_line += 1;
			false
		}
		patch::Line::Remove(_) => false,
	})
}

/// Collect lines per-[patch::Hunk], this is sadly required by the GH issue
/// comment API
pub fn lines_in_same_hunk(hunks: &[patch::Hunk], line1: u64, line2: u64) -> bool {
	for hunk in hunks {
		let start_line = hunk.new_range.start;
		let end_line = hunk.new_range.start + hunk.new_range.count;
		if line1 >= start_line && line1 < end_line && line2 >= start_line && line2 < end_line {
			return true;
		}
	}

	false
}
