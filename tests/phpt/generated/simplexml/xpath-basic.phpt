--TEST--
simplexml: bounded xpath and namespace registration
--DESCRIPTION--
Generated SimpleXML coverage for bounded XPath element/attribute selection and
namespace prefix registration.
--EXTENSIONS--
simplexml
--FILE--
<?php
$xml = simplexml_load_string('<root><item code="a">A</item><item code="b"><title>B</title></item><other>O</other></root>');
$queries = array('item', './item', '/root/item', '//item', 'item/title', './item/title', '/root/item/title', '//title', 'missing', '@code', 'item/@code', '//*');
foreach ($queries as $query) {
    $hits = $xml->xpath($query);
    echo "query=", $query, " count=", count($hits), "\n";
    foreach ($hits as $index => $hit) {
        echo "  ", $index, ":", $hit->getName(), ":", (string) $hit, ":", count($hit), "\n";
    }
}
$attrs = $xml->xpath('item/@code');
foreach ($attrs as $attr) {
    echo "attr name=", $attr->getName(), " str=", (string) $attr, " count=", count($attr), " asxml=", $attr->asXML(), " isset0=", isset($attr[0]) ? "true" : "false", " empty0=", empty($attr[0]) ? "true" : "false", "\n";
}
$ns = simplexml_load_string('<root xmlns:h="urn:h"><h:item id="1">H</h:item></root>');
var_dump($ns->registerXPathNamespace('h', 'urn:h'));
$ns_hits = $ns->xpath('//h:item');
echo "ns hits=", count($ns_hits), " text=", (string) $ns_hits[0], "\n";
?>
--EXPECT--
query=item count=2
  0:item:A:0
  1:item::1
query=./item count=2
  0:item:A:0
  1:item::1
query=/root/item count=2
  0:item:A:0
  1:item::1
query=//item count=2
  0:item:A:0
  1:item::1
query=item/title count=1
  0:title:B:0
query=./item/title count=1
  0:title:B:0
query=/root/item/title count=1
  0:title:B:0
query=//title count=1
  0:title:B:0
query=missing count=0
query=@code count=0
query=item/@code count=2
  0:code:a:1
  1:code:b:1
query=//* count=5
  0:root::3
  1:item:A:0
  2:item::1
  3:title:B:0
  4:other:O:0
attr name=code str=a count=1 asxml= code="a" isset0=true empty0=false
attr name=code str=b count=1 asxml= code="b" isset0=true empty0=false
bool(true)
ns hits=1 text=H
