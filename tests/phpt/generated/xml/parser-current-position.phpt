--TEST--
xml: current parser byte, line, and column helpers
--DESCRIPTION--
Generated XML parser coverage for deterministic current-position helpers after
a successful bounded parse.
--EXTENSIONS--
xml
--FILE--
<?php
$parser = xml_parser_create();
$xml = "<root>\n <child>A</child>\n</root>";
var_dump(xml_parse($parser, $xml, true));
var_dump(strlen($xml));
var_dump(xml_get_current_byte_index($parser));
var_dump(xml_get_current_line_number($parser));
var_dump(xml_get_current_column_number($parser));
?>
--EXPECT--
int(1)
int(32)
int(32)
int(3)
int(8)
