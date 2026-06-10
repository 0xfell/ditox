//! Pure fuzzy-search scoring engine used by storage's `ditox_fuzzy_score`
//! SQLite UDF. No I/O, no allocation: scoring only.

const std = @import("std");

pub fn fuzzyScore(haystack: []const u8, needle_raw: []const u8) i64 {
    const needle = std.mem.trim(u8, needle_raw, " \t\r\n");
    if (haystack.len == 0 or needle.len == 0) return 0;
    return @max(fuzzyScoreMode(haystack, needle, false), fuzzyScoreMode(haystack, needle, true));
}

fn fuzzyScoreMode(haystack: []const u8, needle: []const u8, prefer_boundaries: bool) i64 {
    var search_start: usize = 0;
    var previous_match: ?usize = null;
    var first_match: ?usize = null;
    var last_match: ?usize = null;
    var run: i64 = 0;
    var longest_run: i64 = 0;
    var boundary_matches: i64 = 0;
    var score: i64 = 0;

    for (needle) |needle_byte| {
        const wanted = asciiLower(needle_byte);
        const found = findFuzzyMatch(haystack, wanted, search_start, previous_match, prefer_boundaries);

        const match_index = found orelse return 0;
        if (first_match == null) first_match = match_index;

        if (previous_match != null and match_index == previous_match.? + 1) {
            run += 1;
        } else {
            run = 1;
        }
        longest_run = @max(longest_run, run);

        score += 10 + (run * 12);
        const boundary_match = isMatchBoundary(haystack, match_index);
        if (boundary_match) {
            score += 18;
            boundary_matches += 1;
        }
        if (match_index < 8) score += @intCast(8 - match_index);
        if (previous_match) |previous| {
            const gap: i64 = @intCast(match_index - previous - 1);
            const gap_penalty = if (boundary_match) @min(gap * 2, 16) else @min(gap * 5, 32);
            score -= gap_penalty;
        }

        previous_match = match_index;
        last_match = match_index;
        search_start = match_index + 1;
    }

    if (first_match) |index| score -= @min(@as(i64, @intCast(index)), 24);
    if (first_match != null and last_match != null) {
        const span = last_match.? - first_match.? + 1;
        const span_extra: i64 = @intCast(span - needle.len);
        score -= @min(span_extra * 2, 40);
    }
    if (boundary_matches == @as(i64, @intCast(needle.len))) {
        score += 40 + (@as(i64, @intCast(needle.len)) * 4);
    }
    if (longest_run == @as(i64, @intCast(needle.len))) score += 80;
    score -= @min(@as(i64, @intCast(haystack.len / 24)), 12);
    return @max(score, 1);
}

fn findFuzzyMatch(haystack: []const u8, wanted: u8, search_start: usize, previous_match: ?usize, prefer_boundaries: bool) ?usize {
    var first: ?usize = null;
    var first_boundary: ?usize = null;
    var index = search_start;
    while (index < haystack.len) : (index += 1) {
        if (asciiLower(haystack[index]) != wanted) continue;
        if (previous_match) |previous| {
            if (index == previous + 1) return index;
        }
        if (first == null) first = index;
        if (prefer_boundaries and first_boundary == null and isMatchBoundary(haystack, index)) first_boundary = index;
    }
    return first_boundary orelse first;
}

fn asciiLower(byte: u8) u8 {
    return if (byte >= 'A' and byte <= 'Z') byte + ('a' - 'A') else byte;
}

fn isMatchBoundary(haystack: []const u8, index: usize) bool {
    if (index == 0) return true;
    const previous = haystack[index - 1];
    const current = haystack[index];
    return isWordBoundary(previous) or (isAsciiLower(previous) and isAsciiUpper(current));
}

fn isAsciiLower(byte: u8) bool {
    return byte >= 'a' and byte <= 'z';
}

fn isAsciiUpper(byte: u8) bool {
    return byte >= 'A' and byte <= 'Z';
}

fn isWordBoundary(byte: u8) bool {
    return byte == ' ' or byte == '\t' or byte == '\n' or byte == '\r' or byte == '-' or byte == '_' or byte == '/' or byte == '\\' or byte == '.' or byte == ':';
}

test "fuzzyScore rewards contiguous, acronym, path, and camel-case matches" {
    try std.testing.expect(fuzzyScore("network latency error", "nle") > 0);
    try std.testing.expectEqual(@as(i64, 0), fuzzyScore("totally separate", "nle"));
    try std.testing.expect(fuzzyScore("needle", "nee") > fuzzyScore("n-e-e", "nee"));
    try std.testing.expect(fuzzyScore("network latency error", "nle") > fuzzyScore("annular low effect", "nle"));
    try std.testing.expect(fuzzyScore("src/components/PreviewPane.tsx", "ppt") > fuzzyScore("support pipeline target", "ppt"));
}
