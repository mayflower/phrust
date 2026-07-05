--TEST--
filter array helpers and metadata
--SKIPIF--
<?php if (!extension_loaded("filter")) die("skip filter extension not loaded"); ?>
--FILE--
<?php
var_dump(in_array("int", filter_list(), true));
var_dump(filter_id("int") === FILTER_VALIDATE_INT);
$data = ["a" => "42", "b" => "x", "c" => "1.25"];
var_dump(filter_var_array($data, [
    "a" => FILTER_VALIDATE_INT,
    "b" => ["filter" => FILTER_VALIDATE_INT, "flags" => FILTER_NULL_ON_FAILURE],
    "c" => FILTER_VALIDATE_FLOAT,
]));
var_dump(filter_var(["1", "x"], FILTER_VALIDATE_INT, FILTER_REQUIRE_ARRAY));
var_dump(filter_var("12.3e", FILTER_SANITIZE_NUMBER_FLOAT, FILTER_FLAG_ALLOW_FRACTION | FILTER_FLAG_ALLOW_SCIENTIFIC));
?>
--EXPECT--
bool(true)
bool(true)
array(3) {
  ["a"]=>
  int(42)
  ["b"]=>
  NULL
  ["c"]=>
  float(1.25)
}
array(2) {
  [0]=>
  int(1)
  [1]=>
  bool(false)
}
string(5) "12.3e"
