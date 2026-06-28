--TEST--
wp.stdlib: common arrays, strings, and variables
--DESCRIPTION--
Generated WordPress stdlib harness coverage for framework-style array, string,
and variable helpers already owned by standard modules.
--FILE--
<?php
$merged = array_merge(array("a" => 1), array("b" => 2), array(3));
echo implode(",", array_keys($merged)), "\n";
var_dump(array_values(array("x" => "first", "y" => "second")));
var_dump(in_array("7", array(7), true));
var_dump(array_search("needle", array("hay", "needle", "stack"), true));
var_dump(str_contains("wordpress", "press"));
var_dump(str_starts_with("plugin.php", "plugin"));
var_dump(str_ends_with("theme.zip", ".zip"));
echo trim("  wp core  "), "|", substr("abcdef", 2, 3), "|", strpos("abcdef", "de"), "\n";
printf("%s:%d\n", gettype(array()), strlen("bytes"));
?>
--EXPECT--
a,b,0
array(2) {
  [0]=>
  string(5) "first"
  [1]=>
  string(6) "second"
}
bool(false)
int(1)
bool(true)
bool(true)
bool(true)
wp core|cde|3
array:5
