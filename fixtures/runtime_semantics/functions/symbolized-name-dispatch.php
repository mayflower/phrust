<?php
// runtime-semantics: category=functions expect=pass php_ref_required=1
// Name dispatch across symbolized and dynamic paths must be identical:
// direct calls, dynamic calls, method/static calls, property access,
// literal and dynamic array string keys, builtins, non-ASCII keys.
function greet($who) {
    return "hi $who";
}

class Box {
    public $label = "start";
    public static function stamp($v) {
        return "[$v]";
    }
    public function tag($v) {
        return $this->label . ":" . $v;
    }
}

echo greet("direct"), "\n";

$fn = "greet";
echo $fn("dynamic"), "\n";

$fn2 = "gre" . "et";
echo $fn2("computed"), "\n";

$box = new Box();
echo $box->tag("m1"), "\n";
$method = "tag";
echo $box->$method("m2"), "\n";
echo Box::stamp("s1"), "\n";

$box->label = "written";
echo $box->label, "\n";
$prop = "label";
echo $box->$prop, "\n";

$map = ["alpha" => 1, "beta" => 2, "käse" => 3, "a\x01b" => 4];
echo $map["alpha"], $map["beta"], $map["käse"], $map["a\x01b"], "\n";
$key = "alp" . "ha";
echo $map[$key], "\n";

echo strtoupper("mixed"), "|", strlen("mixed"), "|", implode("-", ["x", "y"]), "\n";
$builtin = "strrev";
echo $builtin("abc"), "\n";
