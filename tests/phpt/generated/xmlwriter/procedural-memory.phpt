--TEST--
xmlwriter: static memory writer and procedural aliases
--DESCRIPTION--
Generated XMLWriter coverage for toMemory(), writeElement(), and memory procedural aliases.
--EXTENSIONS--
xmlwriter
--FILE--
<?php
var_dump(function_exists("xmlwriter_open_memory"));
var_dump(method_exists("XMLWriter", "toMemory"));
var_dump(method_exists("XMLWriter", "writeElement"));

$writer = XMLWriter::toMemory();
var_dump($writer instanceof XMLWriter);
var_dump($writer->startDocument());
var_dump($writer->startElement("root"));
var_dump($writer->writeElement("child", "A & B"));
var_dump($writer->endElement());
echo $writer->outputMemory(), "\n";

$writer = xmlwriter_open_memory();
var_dump($writer instanceof XMLWriter);
var_dump(xmlwriter_start_document($writer));
var_dump(xmlwriter_start_element($writer, "root"));
var_dump(xmlwriter_write_attribute($writer, "id", "7"));
var_dump(xmlwriter_text($writer, "A & B"));
var_dump(xmlwriter_write_element($writer, "child", "C"));
var_dump(xmlwriter_end_document($writer));
echo xmlwriter_output_memory($writer), "\n";
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
<?xml version="1.0"?><root><child>A &amp; B</child></root>
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
<?xml version="1.0"?><root id="7">A &amp; B<child>C</child></root>
