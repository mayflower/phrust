<?php
// runtime-semantics: category=strings expect=pass php_ref_required=1
// String intrinsic fast paths: strtoupper / str_replace / htmlspecialchars /
// explode exact cases plus generic fallbacks - binary and empty strings,
// wrong arity/type, array args, named args, references, diagnostic order.

// strtoupper: mixed case, already-upper (no-change), empty, binary bytes.
echo strtoupper("hello World 123"), "\n";
echo strtoupper("ALREADY"), "|", strtoupper(""), "|", strlen(strtoupper("a\x00\xffz")), "\n";
var_dump(strtoupper("umlaut \u{e4}\u{f6}"));
echo strtoupper(123), "|", strtoupper(true), "\n";
echo strtoupper(string: "named arg"), "\n";

// str_replace: scalar hot path, no-match share, empty search/replace pieces.
echo str_replace("world", "phrust", "hello world, world"), "\n";
echo str_replace("missing", "x", "unchanged subject"), "\n";
echo str_replace("", "x", "empty search"), "|", str_replace("e", "", "delete e"), "\n";
echo bin2hex(str_replace("\x00", "-", "a\x00b\x00c")), "\n";
echo str_replace(["a", "b"], ["1", "2"], "abc"), "\n";
print_r(str_replace("x", "y", ["ax", "bx"]));
$count = 0;
echo str_replace("l", "L", "hello llama", $count), "|", $count, "\n";
echo str_replace(search: "a", replace: "o", subject: "banana"), "\n";
$count2 = 0;
echo str_replace("l", "L", "hello", count: $count2), "|", $count2, "\n";

// htmlspecialchars: default-flag hot path, no-escape share, all five chars.
echo htmlspecialchars("safe text"), "\n";
echo htmlspecialchars("<a href=\"x\">T&C 'quoted'</a>"), "\n";
echo htmlspecialchars(""), "|", htmlspecialchars("uml \u{e4} <b>"), "\n";
echo htmlspecialchars("it's \"quoted\"", ENT_NOQUOTES), "\n";
echo htmlspecialchars("&amp; again", ENT_QUOTES, "UTF-8", false), "\n";
echo htmlspecialchars(string: "<named>"), "\n";
// named arg past defaulted holes (flags/encoding fill from declaration)
echo htmlspecialchars("it's <b>", double_encode: false), "\n";

// explode: single-byte hot path (empty subject, leading/trailing/adjacent
// separators, binary separator); multi-byte separator and limits fall back.
print_r(explode(",", "a,b,c"));
print_r(explode(",", ""));
print_r(explode(",", ",a,,z,"));
print_r(explode("\x00", "a\x00b"));
print_r(explode("::", "a::b::c"));
print_r(explode(",", "a,b,c,d", 2));
print_r(explode(",", "a,b,c,d", -1));
print_r(explode(separator: ".", string: "1.2.3"));

// Diagnostic order: empty separator raises ValueError before splitting.
try {
    explode("", "abc");
} catch (ValueError $e) {
    echo get_class($e), ": ", $e->getMessage(), "\n";
}
try {
    strtoupper();
} catch (ArgumentCountError $e) {
    echo get_class($e), "\n";
}
try {
    explode(",");
} catch (ArgumentCountError $e) {
    echo get_class($e), "\n";
}
try {
    strtoupper([]);
} catch (TypeError $e) {
    echo get_class($e), "\n";
}
