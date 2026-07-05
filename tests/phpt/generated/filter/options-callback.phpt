--TEST--
filter option ranges and callback metadata
--SKIPIF--
<?php if (!extension_loaded("filter")) die("skip filter extension not loaded"); ?>
--FILE--
<?php
var_dump(filter_id("callback") === FILTER_CALLBACK);
var_dump(filter_var("42", FILTER_VALIDATE_INT, ["options" => ["min_range" => 10, "max_range" => 50]]));
var_dump(filter_var("5", FILTER_VALIDATE_INT, ["options" => ["min_range" => 10]]));
var_dump(filter_var("1.25", FILTER_VALIDATE_FLOAT, ["options" => ["min_range" => 1, "max_range" => 2]]));
var_dump(filter_var("2.5", FILTER_VALIDATE_FLOAT, ["options" => ["max_range" => 2]]));
var_dump(filter_var("abc", FILTER_CALLBACK, ["options" => "strtoupper"]));
var_dump(filter_var_array(["a" => "7", "b" => "2"], [
    "a" => ["filter" => FILTER_VALIDATE_INT, "options" => ["min_range" => 5]],
    "b" => ["filter" => FILTER_VALIDATE_INT, "options" => ["min_range" => 5]],
]));
?>
--EXPECT--
bool(true)
int(42)
bool(false)
float(1.25)
bool(false)
string(3) "ABC"
array(2) {
  ["a"]=>
  int(7)
  ["b"]=>
  bool(false)
}
