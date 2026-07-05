--TEST--
xmlreader: navigation and XML string readers
--DESCRIPTION--
Generated XMLReader coverage for bounded next/readString/readInnerXml/readOuterXml support.
--EXTENSIONS--
xmlreader
--FILE--
<?php
$reader = new XMLReader();
var_dump(method_exists("XMLReader", "next"));
var_dump(method_exists("XMLReader", "getAttributeNo"));
var_dump(method_exists("XMLReader", "readString"));
var_dump(method_exists("XMLReader", "readInnerXml"));
var_dump(method_exists("XMLReader", "readOuterXml"));
var_dump($reader->XML('<ns:root xmlns:ns="urn:x" id="7"><child>A</child><child id="8">B</child></ns:root>'));
var_dump($reader->read());
echo $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->depth, "|", $reader->attributeCount, "|", var_export($reader->hasAttributes, true), "|", var_export($reader->getAttributeNo(0), true), "|", $reader->readString(), "|", $reader->readInnerXml(), "|", $reader->readOuterXml(), "\n";
var_dump($reader->read());
echo $reader->nodeType, "|", $reader->name, "|", $reader->depth, "|", $reader->readString(), "|", $reader->readOuterXml(), "\n";
var_dump($reader->next("child"));
echo $reader->nodeType, "|", $reader->name, "|", $reader->depth, "|", var_export($reader->getAttributeNo(0), true), "|", $reader->readString(), "|", $reader->readOuterXml(), "\n";
var_dump($reader->next("missing"));
echo $reader->nodeType, "|", $reader->name, "|", $reader->depth, "\n";
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
1|ns:root|root|ns|0|2|true|'urn:x'|AB|<child>A</child><child id="8">B</child>|<ns:root xmlns:ns="urn:x" id="7"><child>A</child><child id="8">B</child></ns:root>
bool(true)
1|child|1|A|<child>A</child>
bool(true)
1|child|1|'8'|B|<child id="8">B</child>
bool(false)
0||0
