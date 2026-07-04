<?php
// runtime-semantics: category=conversions expect=pass php_ref_required=1
// json_encode fast path: packed/record/nested arrays, scalars, escaped
// strings, plus generic fallbacks for floats, objects, references,
// recursion, options, and unsupported values.

// Scalars and simple strings.
echo json_encode(null), "|", json_encode(true), "|", json_encode(false), "\n";
echo json_encode(0), "|", json_encode(-42), "|", json_encode(PHP_INT_MAX), "\n";
echo json_encode("plain"), "|", json_encode(""), "\n";

// Escaping: quotes, backslash, slash, control chars, unicode, astral.
echo json_encode("say \"hi\" \\ once"), "\n";
echo json_encode("path/to/file"), "\n";
echo json_encode("tab\there\nnewline\rcr\x08b\x0cf\x1fu"), "\n";
echo json_encode("uml \u{e4}\u{f6} euro \u{20ac} astral \u{1F600}"), "\n";

// Packed arrays, nested packed/record shapes.
echo json_encode([1, 2, 3]), "\n";
echo json_encode([]), "\n";
echo json_encode([[1, 2], ["a", "b"], []]), "\n";

// Record/string-key arrays (DTO response shape used by app flows).
$rows = [];
for ($i = 1; $i <= 3; $i++) {
    $rows[] = [
        "id" => $i,
        "name" => "User " . $i,
        "email" => "user{$i}@example.com/inbox",
        "active" => $i % 2 === 1,
        "meta" => ["roles" => ["user", "editor"], "score" => $i * 10],
    ];
}
echo json_encode(["data" => $rows, "total" => 3, "next" => null]), "\n";

// Mixed keys become an object; holes in int keys too.
echo json_encode([5 => "five", "k" => -2]), "\n";
$holey = [0 => "a", 2 => "c"];
echo json_encode($holey), "\n";

// Float values (generic path), zero-fraction floats collapse to ints.
echo json_encode(1.5), "|", json_encode([2.0, 0.25]), "\n";

// Objects go through the generic path.
$obj = new stdClass();
$obj->x = 1;
$obj->y = "two";
echo json_encode($obj), "|", json_encode(["wrap" => $obj]), "\n";

// References are followed; recursive references fail with false.
$shared = ["s" => 1];
$holder = ["a" => &$shared, "b" => &$shared];
echo json_encode($holder), "\n";
$cycle = [];
$cycle["self"] = &$cycle;
var_dump(json_encode($cycle));
echo json_last_error() === JSON_ERROR_RECURSION ? "recursion" : "other", "\n";

// Options fall back to the generic pipeline.
echo json_encode(["path" => "a/b"], JSON_UNESCAPED_SLASHES), "\n";
echo json_encode(["uml" => "\u{e4}"], JSON_UNESCAPED_UNICODE), "\n";
echo json_encode([1, 2], JSON_FORCE_OBJECT), "\n";
echo json_encode(["a" => 1], JSON_PRETTY_PRINT), "\n";
// Explicit default flags value keeps the fast shape.
echo json_encode(["z" => 9], 0), "\n";

// Invalid UTF-8 fails with false + JSON_ERROR_UTF8 (generic path).
var_dump(json_encode("bad \xFF byte"));
echo json_last_error() === JSON_ERROR_UTF8 ? "utf8" : "other", "\n";
// json_last_error resets on the next successful encode.
echo json_encode("ok"), "|", json_last_error(), "\n";
