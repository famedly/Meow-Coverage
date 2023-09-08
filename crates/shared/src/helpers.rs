//! General helper utils

/// Split a path by `source_prefix`, retaining the splitter in the right-paw
/// side
#[must_use]
pub fn path_split(path: &str, source_prefix: &str) -> String {
	path.split_once(source_prefix)
		.map_or_else(|| String::from(path), |(_, val)| format!("{}{}", source_prefix, val))
}

#[cfg(feature = "patch")]
/// Check if a line was changed in a [patch::Hunk]
#[must_use]
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

#[cfg(feature = "patch")]
/// Collect lines per-[patch::Hunk], this is sadly required by the GH issue
/// comment API
#[must_use]
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
