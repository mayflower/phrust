--TEST--
simplexml: array offsets, attributes, and getName
--DESCRIPTION--
Generated SimpleXML coverage for PHP-style attribute offsets, numeric child
selection, attribute iteration, direct isset()/empty(), and getName() metadata.
--EXTENSIONS--
simplexml
--FILE--
<?php
$xml = simplexml_load_string('<root id="7" code="x"><item code="a">A</item><item code="b">B</item><empty/><zero>0</zero><nested><child/></nested></root>');
echo "root attr=", $xml['id'], "\n";
var_dump($xml['missing']);
var_dump($xml['item']);
echo "item0=", $xml->item[0], " name=", $xml->item[0]->getName(), " count=", count($xml->item[0]), "\n";
echo "item1=", $xml->item[1], " name=", $xml->item[1]->getName(), " count=", count($xml->item[1]), "\n";
var_dump($xml->item[2]);
echo "item0 attr=", $xml->item[0]['code'], "\n";
var_dump($xml->item[0]['missing']);
$attrs = $xml->attributes();
echo "attrs=", $attrs, " name=", $attrs->getName(), " count=", count($attrs), "\n";
foreach ($attrs as $name => $value) {
    echo "attr ", $name, "=", $value, " name=", $value->getName(), " count=", count($value), "\n";
}
echo "attrs id dim=", $attrs['id'], "\n";
echo "attrs zero=", $attrs[0], " name=", $attrs[0]->getName(), "\n";
var_dump($attrs[99]);
echo "missing child name=[", $xml->missing->getName(), "] count=", count($xml->missing), " string=[", $xml->missing, "]\n";
echo "isset root attr=", isset($xml['id']) ? "true" : "false", " empty=", empty($xml['id']) ? "true" : "false", "\n";
echo "isset missing attr=", isset($xml['missing']) ? "true" : "false", " empty=", empty($xml['missing']) ? "true" : "false", "\n";
echo "isset item0=", isset($xml->item[0]) ? "true" : "false", " empty=", empty($xml->item[0]) ? "true" : "false", "\n";
echo "isset item2=", isset($xml->item[2]) ? "true" : "false", " empty=", empty($xml->item[2]) ? "true" : "false", "\n";
echo "isset item attr=", isset($xml->item[0]['code']) ? "true" : "false", " empty=", empty($xml->item[0]['code']) ? "true" : "false", "\n";
echo "isset empty child=", isset($xml->empty) ? "true" : "false", " empty=", empty($xml->empty) ? "true" : "false", "\n";
echo "isset empty0=", isset($xml->empty[0]) ? "true" : "false", " empty=", empty($xml->empty[0]) ? "true" : "false", "\n";
echo "isset zero child=", isset($xml->zero) ? "true" : "false", " empty=", empty($xml->zero) ? "true" : "false", "\n";
echo "isset zero0=", isset($xml->zero[0]) ? "true" : "false", " empty=", empty($xml->zero[0]) ? "true" : "false", "\n";
echo "isset nested0=", isset($xml->nested[0]) ? "true" : "false", " empty=", empty($xml->nested[0]) ? "true" : "false", "\n";
echo "isset missing child0=", isset($xml->missing[0]) ? "true" : "false", " empty=", empty($xml->missing[0]) ? "true" : "false", "\n";
?>
--EXPECT--
root attr=7
NULL
NULL
item0=A name=item count=0
item1=B name=item count=0
NULL
item0 attr=a
NULL
attrs=7 name=id count=2
attr id=7 name=id count=0
attr code=x name=code count=0
attrs id dim=7
attrs zero=7 name=id
NULL
missing child name=[] count=0 string=[]
isset root attr=true empty=false
isset missing attr=false empty=true
isset item0=true empty=false
isset item2=false empty=true
isset item attr=true empty=false
isset empty child=true empty=true
isset empty0=true empty=true
isset zero child=true empty=true
isset zero0=true empty=true
isset nested0=true empty=false
isset missing child0=false empty=true
