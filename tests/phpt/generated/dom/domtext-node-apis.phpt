--TEST--
dom: DOMText creation and element append MVP
--DESCRIPTION--
Generated DOM coverage for bounded DOMText nodes, text escaping, and text-node
append behavior inside the XML-backed DOM MVP.
--EXTENSIONS--
dom
--FILE--
<?php
$document = new DOMDocument();
$root = $document->createElement('root');
$text = $document->createTextNode('A & B');
var_dump($text instanceof DOMText);
echo $text->nodeName, "|", $text->nodeValue, "|", $text->textContent, "\n";
$root->appendChild($text);
$document->appendChild($root);
echo $document->saveXML(), "\n";
$direct = new DOMText('direct');
echo $direct->nodeName, "|", $direct->nodeValue, "\n";
?>
--EXPECT--
bool(true)
#text|A & B|A & B
<root>A &amp; B</root>
#text|direct
