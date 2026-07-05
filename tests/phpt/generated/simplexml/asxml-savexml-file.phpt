--TEST--
simplexml: asXML and saveXML filename output
--DESCRIPTION--
Generated SimpleXML coverage for asXML()/saveXML() string output and local file
serialization through the bounded XML tree.
--EXTENSIONS--
simplexml
--FILE--
<?php
function normalized_xml($value) {
    $value = str_replace("<?xml version=\"1.0\"?>\n", "", $value);
    return trim($value);
}

$xml = simplexml_load_string('<root id="7"><child>A &amp; B</child></root>');
$as_path = __DIR__ . "/simplexml-asxml-out.xml";
$save_path = __DIR__ . "/simplexml-savexml-out.xml";
echo "asxml string=", normalized_xml($xml->asXML()), "\n";
echo "savexml string=", normalized_xml($xml->saveXML()), "\n";
var_dump($xml->asXML($as_path));
echo "asxml file=", normalized_xml(file_get_contents($as_path)), "\n";
var_dump($xml->saveXML($save_path));
echo "savexml file=", normalized_xml(file_get_contents($save_path)), "\n";
var_dump(unlink($as_path));
var_dump(unlink($save_path));
?>
--EXPECT--
asxml string=<root id="7"><child>A &amp; B</child></root>
savexml string=<root id="7"><child>A &amp; B</child></root>
bool(true)
asxml file=<root id="7"><child>A &amp; B</child></root>
bool(true)
savexml file=<root id="7"><child>A &amp; B</child></root>
bool(true)
bool(true)
