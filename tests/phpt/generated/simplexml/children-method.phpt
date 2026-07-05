--TEST--
simplexml: children method and duplicate child iteration
--DESCRIPTION--
Generated SimpleXML coverage for children(), child-list string/asXML behavior,
and PHP-style iteration keys when sibling elements share a name.
--EXTENSIONS--
simplexml
--FILE--
<?php
$xml = simplexml_load_string('<root><a>A</a><b>B</b><a>C</a></root>');
$children = $xml->children();
var_dump($children instanceof SimpleXMLElement);
echo $children->asXML(), "\n";
foreach ($children as $name => $value) {
    echo $name, "=", $value, "\n";
}
echo "first-a=", $children->a, "\n";
echo "a-asxml=", $children->a->asXML(), "\n";
foreach ($children->a as $name => $value) {
    echo "a-list ", $name, "=", $value, "\n";
}
?>
--EXPECT--
bool(true)
<a>A</a>
a=A
b=B
a=C
first-a=A
a-asxml=<a>A</a>
a-list a=A
a-list a=C
