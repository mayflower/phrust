--TEST--
xmlreader: open local XML file
--DESCRIPTION--
Generated XMLReader coverage for bounded local-file open support.
--EXTENSIONS--
xmlreader
--FILE--
<?php
$path = tempnam(sys_get_temp_dir(), "phrust_xmlreader_");
file_put_contents($path, '<catalog><item id="9">Book</item></catalog>');

$reader = new XMLReader();
var_dump(method_exists("XMLReader", "open"));
var_dump($reader->open($path));
while ($reader->read()) {
    echo $reader->nodeType, "|", $reader->name, "|", $reader->depth, "|", var_export($reader->getAttribute("id"), true), "|", $reader->readString(), "\n";
}
var_dump($reader->read());
echo $reader->nodeType, "|", $reader->name, "|", $reader->readString(), "\n";
unlink($path);
?>
--EXPECT--
bool(true)
bool(true)
1|catalog|0|NULL|Book
1|item|1|'9'|Book
3|#text|2|NULL|Book
15|item|1|'9'|
15|catalog|0|NULL|
bool(false)
0||
