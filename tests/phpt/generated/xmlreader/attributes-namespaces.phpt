--TEST--
xmlreader: attribute cursor and namespace lookup
--DESCRIPTION--
Generated XMLReader coverage for bounded moveTo* attribute cursor and lookupNamespace support.
--EXTENSIONS--
xmlreader
--FILE--
<?php
$reader = new XMLReader();
var_dump(XMLReader::ATTRIBUTE);
var_dump(method_exists("XMLReader", "lookupNamespace"));
var_dump(method_exists("XMLReader", "moveToFirstAttribute"));
var_dump(method_exists("XMLReader", "moveToNextAttribute"));
var_dump(method_exists("XMLReader", "moveToAttribute"));
var_dump(method_exists("XMLReader", "moveToAttributeNo"));
var_dump(method_exists("XMLReader", "moveToElement"));
var_dump($reader->XML('<ns:root xmlns:ns="urn:x" id="7" ns:flag="yes"><child>A</child></ns:root>'));
var_dump($reader->read());
echo "node=", $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->namespaceURI, "|", $reader->attributeCount, "\n";
var_dump($reader->lookupNamespace("ns"));
var_dump($reader->lookupNamespace("missing"));
var_dump($reader->moveToFirstAttribute());
echo "attr1=", $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->namespaceURI, "|", $reader->value, "|", $reader->depth, "|", var_export($reader->hasValue, true), "\n";
var_dump($reader->moveToNextAttribute());
echo "attr2=", $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->namespaceURI, "|", $reader->value, "|", $reader->depth, "\n";
var_dump($reader->moveToNextAttribute());
echo "attr3=", $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->namespaceURI, "|", $reader->value, "|", $reader->depth, "\n";
var_dump($reader->moveToNextAttribute());
var_dump($reader->moveToElement());
echo "element=", $reader->nodeType, "|", $reader->name, "|", $reader->localName, "|", $reader->prefix, "|", $reader->namespaceURI, "|", $reader->depth, "\n";
var_dump($reader->moveToAttribute("id"));
echo "id=", $reader->nodeType, "|", $reader->name, "|", $reader->value, "\n";
var_dump($reader->moveToAttributeNo(2));
echo "no2=", $reader->nodeType, "|", $reader->name, "|", $reader->value, "\n";
var_dump($reader->moveToElement());
var_dump($reader->read());
echo "afterread=", $reader->nodeType, "|", $reader->name, "|", $reader->depth, "\n";
?>
--EXPECT--
int(2)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
node=1|ns:root|root|ns|urn:x|3
string(5) "urn:x"
NULL
bool(true)
attr1=2|xmlns:ns|ns|xmlns|http://www.w3.org/2000/xmlns/|urn:x|1|true
bool(true)
attr2=2|id|id|||7|1
bool(true)
attr3=2|ns:flag|flag|ns|urn:x|yes|1
bool(false)
bool(true)
element=1|ns:root|root|ns|urn:x|0
bool(true)
id=2|id|7
bool(true)
no2=2|ns:flag|yes
bool(true)
bool(true)
afterread=1|child|1
